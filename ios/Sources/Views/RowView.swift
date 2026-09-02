import SwiftUI

/// Traduit une ligne du modèle de vue en composant SwiftUI, et renvoie les
/// modifications au cœur Rust sous forme d'actions.
struct RowView: View {
    @EnvironmentObject private var transport: Transport
    let row: Row

    @State private var pendingConfirm: Confirm?
    @State private var draft: String = ""

    var body: some View {
        switch row {
        case .text(let label, let value):
            if let value {
                LabeledContent(label, value: value)
            } else {
                Text(label)
            }

        case .picker(let id, let label, let value, let options):
            Picker(label, selection: bind(value) { transport.send(id, text: $0) }) {
                ForEach(options) { Text($0.label).tag($0.value) }
            }

        case .segmented(let id, let value, let options):
            Picker("", selection: bind(value) { transport.send(id, text: $0) }) {
                ForEach(options) { Text($0.label).tag($0.value) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()

        case .toggle(let id, let label, let value):
            Toggle(label, isOn: bind(value) { transport.send(id, bool: $0) })

        case .slider(let id, let label, let value, let min, let max, let step):
            VStack(alignment: .leading) {
                Text(label)
                Slider(
                    value: bind(value) { transport.send(id, number: $0) },
                    in: min...max, step: step)
            }

        case .stepper(let id, let label, let value, let min, let max):
            Stepper(label, value: bind(value) { transport.send(id, number: $0) }, in: min...max)

        case .button(let id, let label, let destructive, let disabled, let confirm):
            Button(label, role: destructive ? .destructive : nil) {
                if let confirm { pendingConfirm = confirm } else { transport.send(id) }
            }
            .disabled(disabled)
            .confirmationDialog(
                pendingConfirm?.title ?? "",
                isPresented: Binding(
                    get: { pendingConfirm != nil },
                    set: { if !$0 { pendingConfirm = nil } }),
                titleVisibility: .visible
            ) {
                Button(pendingConfirm?.actionLabel ?? "Confirmer", role: .destructive) {
                    transport.send(id)
                    pendingConfirm = nil
                }
                Button("Annuler", role: .cancel) { pendingConfirm = nil }
            } message: {
                Text(pendingConfirm?.message ?? "")
            }

        case .field(let id, let label, let value, let placeholder, let secure, let keyboard):
            VStack(alignment: .leading, spacing: 4) {
                Text(label).font(.caption).foregroundStyle(.secondary)
                Group {
                    if secure {
                        SecureField(placeholder, text: $draft)
                    } else {
                        TextField(placeholder, text: $draft)
                            .keyboardType(keyboard == "url" ? .URL : .default)
                    }
                }
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                // La saisie reste locale tant qu'elle n'est pas validée :
                // remonter chaque frappe redessinerait tout l'écran.
                // La valeur initiale vient du modèle ; on ne la renvoie
                // pas, sinon la première apparition déclencherait une action.
                .onAppear { if draft.isEmpty { draft = value } }
                .onChange(of: draft) { old, new in
                    if old != new { transport.send(id, text: new) }
                }
            }

        case .color(let id, let label, let value):
            ColorPicker(
                label,
                selection: Binding(
                    get: { Color(hex: value) },
                    set: { transport.send(id, text: $0.hexString) }),
                supportsOpacity: false)

        case .gauge(let label, let value, let max, let detail):
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(label)
                    Spacer()
                    Text(detail).monospacedDigit().foregroundStyle(.secondary)
                }
                ProgressView(value: value, total: Swift.max(max, 1))
            }
        }
    }

    /// Liaison en écriture seule : la valeur affichée vient toujours du
    /// modèle, jamais d'un état local qui pourrait en diverger.
    private func bind<T>(_ value: T, _ set: @escaping (T) -> Void) -> Binding<T> {
        Binding(get: { value }, set: set)
    }
}

extension Color {
    init(hex: String) {
        let trimmed = hex.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
        guard trimmed.count == 6, let rgb = UInt32(trimmed, radix: 16) else {
            self = .blue
            return
        }
        self.init(
            red: Double((rgb >> 16) & 0xFF) / 255,
            green: Double((rgb >> 8) & 0xFF) / 255,
            blue: Double(rgb & 0xFF) / 255)
    }

    /// `#rrggbb`, borné : les espaces colorimétriques étendus de l'iPhone
    /// peuvent produire des composantes hors de 0…1.
    var hexString: String {
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        UIColor(self).getRed(&r, green: &g, blue: &b, alpha: &a)
        let clamp = { (v: CGFloat) in Int(Swift.max(0, Swift.min(1, v)) * 255) }
        return String(format: "#%02x%02x%02x", clamp(r), clamp(g), clamp(b))
    }
}
