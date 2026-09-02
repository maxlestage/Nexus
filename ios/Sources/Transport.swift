import CoreBluetooth
import Foundation

/// Liaison Bluetooth avec la manette.
///
/// Ne contient aucune logique applicative : elle transmet les événements et
/// les octets au cœur Rust, et écrit ce que celui-ci demande d'émettre. Une
/// seule trame est en vol à la fois — le protocole est un question/réponse.
@MainActor
final class Transport: NSObject, ObservableObject {
    /// Reconstruit à chaque changement d'état, il pilote tout l'affichage.
    @Published private(set) var model: ViewModel?

    private let core = Core()
    private var central: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var rx: CBCharacteristic?

    private let serviceUUID = CBUUID(string: CoreUUIDs.service)
    private let rxUUID = CBUUID(string: CoreUUIDs.rx)
    private let txUUID = CBUUID(string: CoreUUIDs.tx)

    /// Coupe l'attente si la manette ne répond pas.
    private var timeout: Task<Void, Never>?

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: .main)
        refresh()
    }

    // MARK: Cycle interface → Rust → Bluetooth

    /// Transmet une action puis émet ce que le cœur a mis en file.
    func send(_ id: String, json: String = "") {
        core.dispatch(id, json: json)
        if id == "connect" { startScan() }
        if id == "disconnect" { closeConnection() }
        pump()
    }

    func send(_ id: String, text: String) { send(id, json: ActionValue.text(text)) }
    func send(_ id: String, bool: Bool) { send(id, json: ActionValue.bool(bool)) }
    func send(_ id: String, number: Double) { send(id, json: ActionValue.number(number)) }
    func send(_ id: String, index: Int) { send(id, json: ActionValue.number(index)) }

    /// Émet la prochaine trame en attente, s'il y en a une et si la liaison
    /// est libre.
    private func pump() {
        guard timeout == nil, let peripheral, let rx, let payload = core.takeOutgoing() else {
            refresh()
            return
        }
        peripheral.writeValue(payload, for: rx, type: .withResponse)
        timeout = Task { [weak self] in
            try? await Task.sleep(for: .seconds(3))
            guard !Task.isCancelled else { return }
            await self?.reportTimeout()
        }
        refresh()
    }

    private func reportTimeout() {
        timeout = nil
        core.bleError("La manette n'a pas répondu.")
        refresh()
    }

    private func clearTimeout() {
        timeout?.cancel()
        timeout = nil
    }

    /// Relit le modèle de vue : c'est le seul moment où l'écran change.
    private func refresh() { model = core.view() }

    // MARK: Liaison

    private func startScan() {
        guard central.state == .poweredOn else { return }
        core.bleEvent(NEXUS_BLE_SCANNING)
        central.scanForPeripherals(withServices: [serviceUUID])
        refresh()
    }

    private func closeConnection() {
        clearTimeout()
        if let peripheral { central.cancelPeripheralConnection(peripheral) }
        peripheral = nil
        rx = nil
    }
}

// MARK: - CBCentralManagerDelegate

extension Transport: CBCentralManagerDelegate {
    nonisolated func centralManagerDidUpdateState(_ central: CBCentralManager) {
        Task { @MainActor in
            switch central.state {
            case .poweredOn: startScan()
            case .poweredOff: core.bleEvent(NEXUS_BLE_OFF)
            case .unauthorized: core.bleEvent(NEXUS_BLE_UNAUTHORIZED)
            case .unsupported: core.bleEvent(NEXUS_BLE_UNSUPPORTED)
            default: break
            }
            refresh()
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
            core.bleEvent(NEXUS_BLE_CONNECTING)
            central.connect(peripheral)
            refresh()
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
        Task { @MainActor in
            clearTimeout()
            self.peripheral = nil
            rx = nil
            core.bleEvent(NEXUS_BLE_DISCONNECTED)
            refresh()
        }
    }

    nonisolated func centralManager(
        _ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?
    ) {
        Task { @MainActor in
            core.bleError(error?.localizedDescription ?? "Connexion impossible.")
            core.bleEvent(NEXUS_BLE_DISCONNECTED)
            refresh()
        }
    }
}

// MARK: - CBPeripheralDelegate

extension Transport: CBPeripheralDelegate {
    nonisolated func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        Task { @MainActor in
            guard let service = peripheral.services?.first(where: { $0.uuid == serviceUUID })
            else {
                core.bleError("Service de configuration introuvable sur la manette.")
                refresh()
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
                if characteristic.uuid == rxUUID { rx = characteristic }
                if characteristic.uuid == txUUID {
                    peripheral.setNotifyValue(true, for: characteristic)
                }
            }
            guard rx != nil else {
                core.bleError("Caractéristiques Bluetooth manquantes.")
                refresh()
                return
            }
            core.bleEvent(NEXUS_BLE_READY)
            pump()
        }
    }

    nonisolated func peripheral(
        _ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        Task { @MainActor in
            clearTimeout()
            if let error {
                core.bleError(error.localizedDescription)
            } else if let data = characteristic.value {
                core.bleData(data)
            }
            // Une réponse libère la liaison : la trame suivante peut partir.
            pump()
        }
    }

    nonisolated func peripheral(
        _ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic,
        error: Error?
    ) {
        guard let error else { return }
        Task { @MainActor in
            clearTimeout()
            core.bleError(error.localizedDescription)
            pump()
        }
    }
}
