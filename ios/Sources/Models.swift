import Foundation

// Les clés JSON reprennent celles produites par serde côté Rust : la
// configuration transite telle quelle, sans traduction intermédiaire.

/// Bouton logique envoyé à la console.
enum SwitchButton: String, Codable, CaseIterable, Identifiable {
    case a = "A", b = "B", x = "X", y = "Y"
    case l = "L", r = "R", zl = "Zl", zr = "Zr"
    case plus = "Plus", minus = "Minus"
    case lStick = "LStick", rStick = "RStick"
    case home = "Home", capture = "Capture"
    case dpadUp = "DpadUp", dpadDown = "DpadDown"
    case dpadLeft = "DpadLeft", dpadRight = "DpadRight"

    var id: String { rawValue }

    var label: String {
        switch self {
        case .a: return "A"
        case .b: return "B"
        case .x: return "X"
        case .y: return "Y"
        case .l: return "L"
        case .r: return "R"
        case .zl: return "ZL"
        case .zr: return "ZR"
        case .plus: return "+"
        case .minus: return "−"
        case .lStick: return "Clic stick gauche"
        case .rStick: return "Clic stick droit"
        case .home: return "Home"
        case .capture: return "Capture"
        case .dpadUp: return "Croix ↑"
        case .dpadDown: return "Croix ↓"
        case .dpadLeft: return "Croix ←"
        case .dpadRight: return "Croix →"
        }
    }

    /// Position du bouton dans le masque de bits du turbo (ordre de l'énumération Rust).
    var bit: UInt32 {
        let order: [SwitchButton] = [
            .y, .x, .b, .a, .r, .zr, .l, .zl, .minus, .plus,
            .rStick, .lStick, .home, .capture, .dpadUp, .dpadDown, .dpadLeft, .dpadRight,
        ]
        return 1 << UInt32(order.firstIndex(of: self) ?? 0)
    }
}

/// Entrée physique de la manette. L'ordre est celui du firmware.
enum PhysicalInput: Int, CaseIterable, Identifiable {
    case faceTop = 0, faceRight, faceBottom, faceLeft
    case indexUpper, indexLower, middleUpper, middleLower
    case palm, stickClick, plus, minus, home, capture
    case turboMod, shiftMod

    var id: Int { rawValue }

    /// Un modificateur pilote la manette elle-même : il n'est pas remappable.
    var isModifier: Bool { self == .turboMod || self == .shiftMod }

    var label: String {
        switch self {
        case .faceTop: return "Pouce · haut"
        case .faceRight: return "Pouce · droite"
        case .faceBottom: return "Pouce · bas"
        case .faceLeft: return "Pouce · gauche"
        case .indexUpper: return "Index · gâchette haute"
        case .indexLower: return "Index · gâchette basse"
        case .middleUpper: return "Majeur · gâchette haute"
        case .middleLower: return "Majeur · gâchette basse"
        case .palm: return "Paume"
        case .stickClick: return "Clic du stick"
        case .plus: return "Bouton +"
        case .minus: return "Bouton −"
        case .home: return "Home"
        case .capture: return "Capture"
        case .turboMod: return "Modificateur TURBO"
        case .shiftMod: return "Modificateur SHIFT"
        }
    }
}

enum StickTarget: String, Codable { case left = "Left", right = "Right" }

enum LedMode: String, Codable, CaseIterable, Identifiable {
    case off = "Off", solid = "Solid", breathe = "Breathe"
    case rainbow = "Rainbow", react = "React"

    var id: String { rawValue }

    var label: String {
        switch self {
        case .off: return "Éteint"
        case .solid: return "Fixe"
        case .breathe: return "Respiration"
        case .rainbow: return "Arc-en-ciel"
        case .react: return "Réagit aux appuis"
        }
    }
}

struct TurboConfig: Codable, Equatable {
    var rateHz: UInt8
    var enabledMask: UInt32

    enum CodingKeys: String, CodingKey {
        case rateHz = "rate_hz"
        case enabledMask = "enabled_mask"
    }
}

struct LedConfig: Codable, Equatable {
    var mode: LedMode
    var r: UInt8
    var g: UInt8
    var b: UInt8
    var brightness: UInt8
}

struct HapticConfig: Codable, Equatable {
    var enabled: Bool
    var strength: UInt8
    var clickOnPress: Bool

    enum CodingKeys: String, CodingKey {
        case enabled, strength
        case clickOnPress = "click_on_press"
    }
}

struct MacroStep: Codable, Equatable {
    var buttonsMask: UInt32
    var durationMs: UInt16

    enum CodingKeys: String, CodingKey {
        case buttonsMask = "buttons_mask"
        case durationMs = "duration_ms"
    }
}

struct MacroDef: Codable, Equatable, Identifiable {
    var triggerMask: UInt16
    var steps: [MacroStep]

    var id: String { "\(triggerMask)-\(steps.count)" }

    enum CodingKeys: String, CodingKey {
        case triggerMask = "trigger_mask"
        case steps
    }

    /// Une combinaison d'entrées physiques déclenchant un bouton unique.
    static func chord(_ inputs: [PhysicalInput], to button: SwitchButton, holdMs: UInt16 = 60) -> MacroDef {
        let mask = inputs.reduce(UInt16(0)) { $0 | (1 << UInt16($1.rawValue)) }
        return MacroDef(
            triggerMask: mask,
            steps: [MacroStep(buttonsMask: button.bit, durationMs: holdMs)])
    }

    var triggerInputs: [PhysicalInput] {
        PhysicalInput.allCases.filter { triggerMask & (1 << UInt16($0.rawValue)) != 0 }
    }
}

struct ControllerConfig: Codable, Equatable {
    var version: UInt8
    var layerNormal: [SwitchButton?]
    var layerShift: [SwitchButton?]
    var stickNormal: StickTarget
    var stickShift: StickTarget
    var turbo: TurboConfig
    var macros: [MacroDef]
    var leds: LedConfig
    var haptics: HapticConfig
    var stickDeadzone: UInt16

    enum CodingKeys: String, CodingKey {
        case version
        case layerNormal = "layer_normal"
        case layerShift = "layer_shift"
        case stickNormal = "stick_normal"
        case stickShift = "stick_shift"
        case turbo, macros, leds, haptics
        case stickDeadzone = "stick_deadzone"
    }

    /// Bouton associé à une entrée, sur la couche demandée.
    func mapping(_ input: PhysicalInput, shift: Bool) -> SwitchButton? {
        let layer = shift ? layerShift : layerNormal
        guard input.rawValue < layer.count else { return nil }
        return layer[input.rawValue]
    }

    mutating func setMapping(_ input: PhysicalInput, shift: Bool, to button: SwitchButton?) {
        guard input.rawValue < layerNormal.count else { return }
        if shift { layerShift[input.rawValue] = button } else { layerNormal[input.rawValue] = button }
    }
}

struct ControllerStats: Codable, Equatable {
    var presses: [UInt32]
    var uptimeS: UInt32
    var macrosFired: UInt32

    enum CodingKeys: String, CodingKey {
        case presses
        case uptimeS = "uptime_s"
        case macrosFired = "macros_fired"
    }

    static let empty = ControllerStats(
        presses: Array(repeating: 0, count: PhysicalInput.allCases.count),
        uptimeS: 0, macrosFired: 0)
}

/// Réponse de la manette, telle que le pont Rust la restitue.
enum ControllerResponse: Decodable {
    case info(protocolVersion: Int, firmwareVersion: String, name: String)
    case config(ControllerConfig)
    case stats(ControllerStats)
    case battery(millivolts: Int, percent: Int, charging: Bool)
    case otaProgress(Int)
    case ok
    case error(String)

    private enum CodingKeys: String, CodingKey {
        case kind, protocolVersion, firmwareVersion, name
        case config, stats, millivolts, percent, charging, code
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .kind) {
        case "info":
            self = .info(
                protocolVersion: try c.decode(Int.self, forKey: .protocolVersion),
                firmwareVersion: try c.decode(String.self, forKey: .firmwareVersion),
                name: try c.decode(String.self, forKey: .name))
        case "config":
            self = .config(try c.decode(ControllerConfig.self, forKey: .config))
        case "stats":
            self = .stats(try c.decode(ControllerStats.self, forKey: .stats))
        case "battery":
            self = .battery(
                millivolts: try c.decode(Int.self, forKey: .millivolts),
                percent: try c.decode(Int.self, forKey: .percent),
                charging: try c.decode(Bool.self, forKey: .charging))
        case "otaProgress":
            self = .otaProgress(try c.decode(Int.self, forKey: .percent))
        case "ok":
            self = .ok
        case "error":
            self = .error(try c.decode(String.self, forKey: .code))
        case let other:
            throw DecodingError.dataCorruptedError(
                forKey: .kind, in: c, debugDescription: "réponse inconnue : \(other)")
        }
    }
}
