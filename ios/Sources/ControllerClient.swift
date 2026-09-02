import Combine
import CoreBluetooth
import Foundation

/// Client Bluetooth de la manette.
///
/// Une seule requête est en vol à la fois : le protocole est un simple
/// question/réponse, et la manette n'a pas de file d'attente. Les réponses
/// spontanées (progression OTA) sont publiées sans consommer de requête.
@MainActor
final class ControllerClient: NSObject, ObservableObject {
    @Published private(set) var state: ConnectionState = .idle
    @Published private(set) var firmwareVersion: String?
    @Published private(set) var battery: Battery?
    @Published private(set) var otaProgress: Int?
    @Published var config: ControllerConfig?
    @Published private(set) var stats: ControllerStats?
    @Published var lastError: String?

    enum ConnectionState: Equatable {
        case idle, scanning, connecting, ready
        case unavailable(String)

        var isReady: Bool { self == .ready }
    }

    struct Battery: Equatable {
        var millivolts: Int
        var percent: Int
        var charging: Bool
        var volts: String { String(format: "%.2f V", Double(millivolts) / 1000) }
    }

    private var central: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var rxCharacteristic: CBCharacteristic?
    private var txCharacteristic: CBCharacteristic?

    /// Requête en attente de sa réponse.
    private var pending: CheckedContinuation<ControllerResponse, Error>?
    private var pendingTimeout: Task<Void, Never>?

    private let serviceUUID = CBUUID(string: Bridge.serviceUUID)
    private let rxUUID = CBUUID(string: Bridge.rxCharUUID)
    private let txUUID = CBUUID(string: Bridge.txCharUUID)

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: .main)
    }

    // MARK: Connexion

    func startScan() {
        guard central.state == .poweredOn else {
            state = .unavailable("Activez le Bluetooth pour connecter la manette.")
            return
        }
        state = .scanning
        central.scanForPeripherals(withServices: [serviceUUID])
    }

    func disconnect() {
        if let peripheral { central.cancelPeripheralConnection(peripheral) }
        teardown()
    }

    private func teardown() {
        failPending(BridgeError.rust("Connexion perdue — la manette est-elle allumée ?"))
        peripheral = nil
        rxCharacteristic = nil
        txCharacteristic = nil
        firmwareVersion = nil
        otaProgress = nil
        state = central?.state == .poweredOn ? .idle : state
    }

    // MARK: Requêtes

    /// Envoie une requête déjà encodée et attend la réponse correspondante.
    @discardableResult
    private func send(_ payload: Data) async throws -> ControllerResponse {
        guard let peripheral, let rx = rxCharacteristic else {
            throw BridgeError.rust("Manette non connectée.")
        }
        if pending != nil {
            throw BridgeError.rust("Une requête est déjà en cours.")
        }
        return try await withCheckedThrowingContinuation { continuation in
            pending = continuation
            peripheral.writeValue(payload, for: rx, type: .withResponse)
            pendingTimeout = Task { [weak self] in
                try? await Task.sleep(for: .seconds(3))
                guard !Task.isCancelled else { return }
                await self?.failPending(BridgeError.rust("La manette n'a pas répondu."))
            }
        }
    }

    private func resolvePending(_ response: ControllerResponse) {
        pendingTimeout?.cancel()
        pendingTimeout = nil
        pending?.resume(returning: response)
        pending = nil
    }

    private func failPending(_ error: Error) {
        pendingTimeout?.cancel()
        pendingTimeout = nil
        pending?.resume(throwing: error)
        pending = nil
    }

    /// Exécute une action réseau en remontant proprement l'erreur à l'écran.
    private func perform(_ body: () async throws -> Void) async {
        do {
            lastError = nil
            try await body()
        } catch {
            lastError = error.localizedDescription
        }
    }

    func refreshEverything() async {
        await perform {
            if case .info(_, let firmware, _) = try await send(Bridge.encode(.getInfo)) {
                firmwareVersion = firmware
            }
            if case .config(let c) = try await send(Bridge.encode(.getConfig)) {
                config = c
            }
            try await refreshBatteryThrowing()
        }
    }

    private func refreshBatteryThrowing() async throws {
        if case .battery(let mv, let pct, let charging) = try await send(Bridge.encode(.getBattery)) {
            battery = Battery(millivolts: mv, percent: pct, charging: charging)
        }
    }

    func refreshBattery() async { await perform { try await refreshBatteryThrowing() } }

    func refreshStats() async {
        await perform {
            if case .stats(let s) = try await send(Bridge.encode(.getStats)) { stats = s }
        }
    }

    func resetStats() async {
        await perform {
            _ = try await send(Bridge.encode(.resetStats))
            if case .stats(let s) = try await send(Bridge.encode(.getStats)) { stats = s }
        }
    }

    /// Applique la configuration à chaud (sans l'écrire en mémoire flash).
    func apply(_ newConfig: ControllerConfig) async {
        config = newConfig
        await perform { _ = try await send(Bridge.encodeSetConfig(newConfig)) }
    }

    func save() async {
        await perform { _ = try await send(Bridge.encode(.saveConfig)) }
    }

    func factoryReset() async {
        await perform {
            _ = try await send(Bridge.encode(.factoryReset))
            if case .config(let c) = try await send(Bridge.encode(.getConfig)) { config = c }
        }
    }

    func identify() async {
        await perform { _ = try await send(Bridge.encode(.identify)) }
    }

    func testHaptic() async {
        await perform { _ = try await send(Bridge.encodeTestHaptic(effect: 1)) }
    }

    func startOTA(ssid: String, password: String, url: String) async {
        await perform {
            _ = try await send(Bridge.encodeStartOTA(ssid: ssid, password: password, url: url))
            otaProgress = 0
        }
    }
}

// MARK: - CBCentralManagerDelegate

extension ControllerClient: CBCentralManagerDelegate {
    nonisolated func centralManagerDidUpdateState(_ central: CBCentralManager) {
        Task { @MainActor in
            switch central.state {
            case .poweredOn:
                if state == .idle { startScan() }
            case .unauthorized:
                state = .unavailable("Autorisez le Bluetooth dans Réglages pour cette app.")
            case .poweredOff:
                state = .unavailable("Le Bluetooth est désactivé.")
            case .unsupported:
                state = .unavailable("Cet appareil ne gère pas le Bluetooth LE.")
            default:
                break
            }
        }
    }

    nonisolated func centralManager(
        _ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
        advertisementData: [String: Any], rssi RSSI: NSNumber
    ) {
        Task { @MainActor in
            central.stopScan()
            self.peripheral = peripheral
            peripheral.delegate = self
            state = .connecting
            central.connect(peripheral)
        }
    }

    nonisolated func centralManager(
        _ central: CBCentralManager, didConnect peripheral: CBPeripheral
    ) {
        Task { @MainActor in peripheral.discoverServices([serviceUUID]) }
    }

    nonisolated func centralManager(
        _ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral,
        error: Error?
    ) {
        Task { @MainActor in teardown() }
    }

    nonisolated func centralManager(
        _ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?
    ) {
        Task { @MainActor in
            lastError = error?.localizedDescription ?? "Connexion impossible."
            teardown()
        }
    }
}

// MARK: - CBPeripheralDelegate

extension ControllerClient: CBPeripheralDelegate {
    nonisolated func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        Task { @MainActor in
            guard let service = peripheral.services?.first(where: { $0.uuid == serviceUUID }) else {
                lastError = "Service de configuration introuvable sur la manette."
                return
            }
            peripheral.discoverCharacteristics([rxUUID, txUUID], for: service)
        }
    }

    nonisolated func peripheral(
        _ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService,
        error: Error?
    ) {
        Task { @MainActor in
            for characteristic in service.characteristics ?? [] {
                if characteristic.uuid == rxUUID { rxCharacteristic = characteristic }
                if characteristic.uuid == txUUID {
                    txCharacteristic = characteristic
                    peripheral.setNotifyValue(true, for: characteristic)
                }
            }
            guard rxCharacteristic != nil, txCharacteristic != nil else {
                lastError = "Caractéristiques BLE manquantes."
                return
            }
            state = .ready
            await refreshEverything()
        }
    }

    nonisolated func peripheral(
        _ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        Task { @MainActor in
            if let error {
                failPending(error)
                return
            }
            guard let data = characteristic.value else { return }
            do {
                let response = try Bridge.decodeResponse(data)
                // La progression OTA arrive spontanément : elle ne répond à
                // aucune requête et ne doit donc pas en consommer une.
                if case .otaProgress(let percent) = response {
                    otaProgress = percent >= 100 ? nil : percent
                    return
                }
                resolvePending(response)
            } catch {
                failPending(error)
            }
        }
    }

    nonisolated func peripheral(
        _ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        if let error { Task { @MainActor in failPending(error) } }
    }
}
