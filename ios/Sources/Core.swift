import Foundation

/// Accès au cœur Rust de l'application.
///
/// Cette classe ne décide de rien : elle transmet les actions, réinjecte ce
/// qui arrive du Bluetooth, et relit le modèle de vue que Rust produit.
@MainActor
final class Core {
    private let handle: OpaquePointer
    private var buffer = [UInt8](repeating: 0, count: 64 * 1024)

    init() {
        guard let handle = nexus_app_new() else {
            fatalError("le cœur de l'application n'a pas pu être créé")
        }
        self.handle = handle
    }

    deinit { nexus_app_free(handle) }

    // MARK: Utilitaires

    private func lastError() -> String {
        var buf = [UInt8](repeating: 0, count: 1024)
        let n = nexus_last_error(&buf, buf.count)
        guard n > 0 else { return "erreur inconnue" }
        return String(decoding: buf[0..<Int(n)], as: UTF8.self)
    }

    /// Exécute un appel produisant des octets. `nil` si rien n'est produit.
    private func bytes(_ body: (UnsafeMutablePointer<UInt8>, Int) -> Int) -> Data? {
        let n = buffer.withUnsafeMutableBufferPointer { body($0.baseAddress!, $0.count) }
        if n > 0 { return Data(buffer[0..<n]) }
        if n < 0 { print("nexus: \(lastError())") }
        return nil
    }

    // MARK: Interface

    /// Modèle de vue courant. Toute l'interface en découle.
    func view() -> ViewModel? {
        guard let json = bytes({ nexus_app_view(handle, $0, $1) }) else { return nil }
        do {
            return try JSONDecoder().decode(ViewModel.self, from: json)
        } catch {
            print("nexus: modèle de vue illisible — \(error)")
            return nil
        }
    }

    /// Transmet une action. `json` porte la valeur ; vide pour un bouton.
    func dispatch(_ id: String, json: String = "") {
        let idBytes = Array(id.utf8)
        let valueBytes = Array(json.utf8)
        _ = idBytes.withUnsafeBufferPointer { idPtr in
            valueBytes.withUnsafeBufferPointer { valuePtr in
                nexus_app_dispatch(
                    handle, idPtr.baseAddress, idBytes.count,
                    valuePtr.baseAddress, valueBytes.count)
            }
        }
    }

    // MARK: Bluetooth

    func bleEvent(_ event: NexusBleEvent) {
        _ = nexus_app_ble_event(handle, event.rawValue)
    }

    func bleData(_ data: Data) {
        _ = data.withUnsafeBytes { raw in
            nexus_app_ble_data(handle, raw.bindMemory(to: UInt8.self).baseAddress, data.count)
        }
    }

    func bleError(_ message: String) {
        let bytes = Array(message.utf8)
        _ = bytes.withUnsafeBufferPointer {
            nexus_app_ble_error(handle, $0.baseAddress, bytes.count)
        }
    }

    /// Prochaine trame à écrire sur la caractéristique RX, s'il y en a une.
    func takeOutgoing() -> Data? {
        bytes { nexus_app_take_outgoing(handle, $0, $1) }
    }

}

/// Identifiants du service BLE, lus depuis Rust plutôt que recopiés : une
/// divergence resterait invisible jusqu'à ce que la manette soit introuvable.
/// Hors de `Core`, qui est cantonné au fil principal.
enum CoreUUIDs {
    private static func read(_ body: (UnsafeMutablePointer<UInt8>, Int) -> Int) -> String {
        var buf = [UInt8](repeating: 0, count: 64)
        let n = buf.withUnsafeMutableBufferPointer { body($0.baseAddress!, $0.count) }
        return n > 0 ? String(decoding: buf[0..<n], as: UTF8.self) : ""
    }

    static let service = read { nexus_service_uuid($0, $1) }
    static let rx = read { nexus_rx_char_uuid($0, $1) }
    static let tx = read { nexus_tx_char_uuid($0, $1) }
}

/// Encodage JSON des valeurs d'action, sans passer par `Codable` : les
/// quatre formes utilisées par le modèle de vue suffisent.
enum ActionValue {
    static func text(_ value: String) -> String {
        let escaped = value
            .replacingOccurrences(of: "\\", with: "\\\\")
            .replacingOccurrences(of: "\"", with: "\\\"")
            .replacingOccurrences(of: "\n", with: "\\n")
            .replacingOccurrences(of: "\t", with: "\\t")
            .replacingOccurrences(of: "\r", with: "\\r")
        return "\"\(escaped)\""
    }

    static func bool(_ value: Bool) -> String { value ? "true" : "false" }
    static func number(_ value: Double) -> String { String(value) }
    static func number(_ value: Int) -> String { String(value) }
}
