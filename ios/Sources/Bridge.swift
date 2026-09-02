import Foundation

/// Erreurs remontées par le pont Rust.
enum BridgeError: LocalizedError {
    case rust(String)

    var errorDescription: String? {
        switch self {
        case .rust(let message): return message
        }
    }
}

/// Enveloppe Swift des fonctions C de `controller-core`.
///
/// L'application ne connaît jamais l'encodage `postcard` du protocole : elle
/// manipule du JSON, et c'est le code Rust — celui-là même qui tourne dans le
/// firmware — qui produit et relit les octets envoyés en Bluetooth.
enum Bridge {
    /// Capacité des tampons d'échange : large devant la taille d'un message.
    private static let bufferSize = 16 * 1024

    private static func lastError() -> String {
        var buf = [UInt8](repeating: 0, count: 1024)
        let n = nexus_last_error(&buf, buf.count)
        guard n > 0 else { return "erreur inconnue du pont" }
        return String(decoding: buf[0..<Int(n)], as: UTF8.self)
    }

    /// Exécute une fonction du pont et renvoie les octets produits.
    private static func call(
        _ body: (UnsafeMutablePointer<UInt8>, Int) -> Int
    ) throws -> Data {
        var buf = [UInt8](repeating: 0, count: bufferSize)
        let n = buf.withUnsafeMutableBufferPointer { body($0.baseAddress!, $0.count) }
        guard n >= 0 else { throw BridgeError.rust(lastError()) }
        return Data(buf[0..<n])
    }

    private static func callString(
        _ body: (UnsafeMutablePointer<UInt8>, Int) -> Int
    ) throws -> String {
        String(decoding: try call(body), as: UTF8.self)
    }

    // MARK: Identifiants BLE

    /// Lus depuis Rust plutôt que recopiés : une divergence serait invisible
    /// jusqu'à ce que l'application ne trouve plus la manette.
    static var serviceUUID: String { (try? callString { nexus_service_uuid($0, $1) }) ?? "" }
    static var rxCharUUID: String { (try? callString { nexus_rx_char_uuid($0, $1) }) ?? "" }
    static var txCharUUID: String { (try? callString { nexus_tx_char_uuid($0, $1) }) ?? "" }

    // MARK: Requêtes

    enum SimpleRequest: UInt32 {
        case getInfo = 0, getConfig = 1, saveConfig = 2, factoryReset = 3
        case getStats = 4, resetStats = 5, identify = 6, getBattery = 7
    }

    static func encode(_ request: SimpleRequest) throws -> Data {
        try call { nexus_encode_simple_request(request.rawValue, $0, $1) }
    }

    static func encodeSetConfig(_ config: ControllerConfig) throws -> Data {
        let json = try JSONEncoder().encode(config)
        return try call { out, cap in
            json.withUnsafeBytes { raw in
                nexus_encode_set_config(
                    raw.bindMemory(to: UInt8.self).baseAddress, json.count, out, cap)
            }
        }
    }

    static func encodeTestHaptic(effect: UInt8) throws -> Data {
        try call { nexus_encode_test_haptic(effect, $0, $1) }
    }

    static func encodeStartOTA(ssid: String, password: String, url: String) throws -> Data {
        let s = Array(ssid.utf8), p = Array(password.utf8), u = Array(url.utf8)
        return try call { out, cap in
            nexus_encode_start_ota(s, s.count, p, p.count, u, u.count, out, cap)
        }
    }

    // MARK: Réponses

    static func decodeResponse(_ data: Data) throws -> ControllerResponse {
        let json = try call { out, cap in
            data.withUnsafeBytes { raw in
                nexus_decode_response(
                    raw.bindMemory(to: UInt8.self).baseAddress, data.count, out, cap)
            }
        }
        return try JSONDecoder().decode(ControllerResponse.self, from: json)
    }

    // MARK: Gabarits

    static func defaultConfig() throws -> ControllerConfig {
        try JSONDecoder().decode(
            ControllerConfig.self, from: try call { nexus_default_config_json($0, $1) })
    }
}
