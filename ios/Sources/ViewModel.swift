import Foundation

// Miroir Swift du modèle de vue produit par Rust. Ces types ne décident de
// rien : ils décrivent ce qu'il faut dessiner.

struct ViewModel: Decodable {
    let screen: Screen
    let banner: Banner?
    let error: String?
    let busy: Bool
}

enum Screen: Decodable {
    case connect(title: String, message: String, action: Row?, spinner: Bool)
    case tabs([Tab])

    private enum CodingKeys: String, CodingKey {
        case kind, title, message, action, spinner, tabs
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .kind) {
        case "connect":
            self = .connect(
                title: try c.decode(String.self, forKey: .title),
                message: try c.decode(String.self, forKey: .message),
                action: try c.decodeIfPresent(Row.self, forKey: .action),
                spinner: try c.decode(Bool.self, forKey: .spinner))
        case "tabs":
            self = .tabs(try c.decode([Tab].self, forKey: .tabs))
        case let other:
            throw DecodingError.dataCorruptedError(
                forKey: .kind, in: c, debugDescription: "écran inconnu : \(other)")
        }
    }
}

struct Tab: Decodable, Identifiable {
    let id: String
    let title: String
    let icon: String
    let selected: Bool
    let sections: [Section]
}

struct Section: Decodable, Identifiable {
    let header: String?
    let footer: String?
    let rows: [Row]

    /// Les sections n'ont pas d'identifiant propre : celui de leur première
    /// ligne suffit à les distinguer dans une liste.
    var id: String { (header ?? "") + (rows.first?.id ?? "") }
}

struct Choice: Decodable, Identifiable, Hashable {
    let value: String
    let label: String
    var id: String { value }
}

struct Confirm: Decodable {
    let title: String
    let message: String
    let actionLabel: String

    private enum CodingKeys: String, CodingKey {
        case title, message
        case actionLabel = "action_label"
    }
}

enum Banner: Decodable {
    case ota(percent: Int, title: String, message: String)

    private enum CodingKeys: String, CodingKey { case kind, percent, title, message }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .kind) {
        case "ota":
            self = .ota(
                percent: try c.decode(Int.self, forKey: .percent),
                title: try c.decode(String.self, forKey: .title),
                message: try c.decode(String.self, forKey: .message))
        case let other:
            throw DecodingError.dataCorruptedError(
                forKey: .kind, in: c, debugDescription: "bandeau inconnu : \(other)")
        }
    }
}

enum Row: Decodable, Identifiable {
    case text(label: String, value: String?)
    case picker(id: String, label: String, value: String, options: [Choice])
    case toggle(id: String, label: String, value: Bool)
    case slider(id: String, label: String, value: Double, min: Double, max: Double, step: Double)
    case stepper(id: String, label: String, value: Double, min: Double, max: Double)
    case button(id: String, label: String, destructive: Bool, disabled: Bool, confirm: Confirm?)
    case field(
        id: String, label: String, value: String, placeholder: String, secure: Bool,
        keyboard: String)
    case color(id: String, label: String, value: String)
    case gauge(label: String, value: Double, max: Double, detail: String)
    case segmented(id: String, value: String, options: [Choice])

    private enum CodingKeys: String, CodingKey {
        case type, id, label, value, options, min, max, step
        case destructive, disabled, confirm, placeholder, secure, keyboard, detail
    }

    var id: String {
        switch self {
        case .text(let label, _): return "text:" + label
        case .gauge(let label, _, _, _): return "gauge:" + label
        case .picker(let id, _, _, _), .toggle(let id, _, _),
            .slider(let id, _, _, _, _, _), .stepper(let id, _, _, _, _),
            .button(let id, _, _, _, _), .field(let id, _, _, _, _, _),
            .color(let id, _, _), .segmented(let id, _, _):
            return id
        }
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let type = try c.decode(String.self, forKey: .type)
        switch type {
        case "text":
            self = .text(
                label: try c.decode(String.self, forKey: .label),
                value: try c.decodeIfPresent(String.self, forKey: .value))
        case "picker":
            self = .picker(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(String.self, forKey: .value),
                options: try c.decode([Choice].self, forKey: .options))
        case "toggle":
            self = .toggle(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(Bool.self, forKey: .value))
        case "slider":
            self = .slider(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(Double.self, forKey: .value),
                min: try c.decode(Double.self, forKey: .min),
                max: try c.decode(Double.self, forKey: .max),
                step: try c.decode(Double.self, forKey: .step))
        case "stepper":
            self = .stepper(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(Double.self, forKey: .value),
                min: try c.decode(Double.self, forKey: .min),
                max: try c.decode(Double.self, forKey: .max))
        case "button":
            self = .button(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                destructive: try c.decode(Bool.self, forKey: .destructive),
                disabled: try c.decode(Bool.self, forKey: .disabled),
                confirm: try c.decodeIfPresent(Confirm.self, forKey: .confirm))
        case "field":
            self = .field(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(String.self, forKey: .value),
                placeholder: try c.decode(String.self, forKey: .placeholder),
                secure: try c.decode(Bool.self, forKey: .secure),
                keyboard: try c.decode(String.self, forKey: .keyboard))
        case "color":
            self = .color(
                id: try c.decode(String.self, forKey: .id),
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(String.self, forKey: .value))
        case "gauge":
            self = .gauge(
                label: try c.decode(String.self, forKey: .label),
                value: try c.decode(Double.self, forKey: .value),
                max: try c.decode(Double.self, forKey: .max),
                detail: try c.decode(String.self, forKey: .detail))
        case "segmented":
            self = .segmented(
                id: try c.decode(String.self, forKey: .id),
                value: try c.decode(String.self, forKey: .value),
                options: try c.decode([Choice].self, forKey: .options))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: c, debugDescription: "ligne inconnue : \(type)")
        }
    }
}
