import SwiftUI

/// Une combinaison de boutons physiques déclenche un bouton logique.
struct MacrosView: View {
    @EnvironmentObject private var client: ControllerClient
    @State private var chord: Set<PhysicalInput> = []
    @State private var output: SwitchButton = .x

    private var remappable: [PhysicalInput] {
        PhysicalInput.allCases.filter { !$0.isModifier }
    }

    /// Le firmware borne le nombre de macros pour que la configuration tienne
    /// dans un message Bluetooth.
    private let maxMacros = 4

    var body: some View {
        NavigationStack {
            Form {
                if let config = client.config {
                    Section("Macros enregistrées") {
                        if config.macros.isEmpty {
                            Text("Aucune macro.").foregroundStyle(.secondary)
                        }
                        ForEach(Array(config.macros.enumerated()), id: \.offset) { index, macro in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(describe(macro)).font(.body)
                                Text(macro.triggerInputs.map(\.label).joined(separator: " + "))
                                    .font(.caption).foregroundStyle(.secondary)
                            }
                            .swipeActions {
                                Button("Supprimer", role: .destructive) { remove(at: index) }
                            }
                        }
                    }

                    Section("Nouvelle macro") {
                        ForEach(remappable) { input in
                            Toggle(input.label, isOn: chordBinding(for: input))
                        }
                        Picker("Bouton émis", selection: $output) {
                            ForEach(SwitchButton.allCases) { Text($0.label).tag($0) }
                        }
                        Button("Ajouter la macro") { add() }
                            .disabled(chord.count < 2 || config.macros.count >= maxMacros)
                    }
                    if config.macros.count >= maxMacros {
                        Text("Limite de \(maxMacros) macros atteinte.")
                            .font(.footnote).foregroundStyle(.secondary)
                    }
                }
                SaveSection()
            }
            .navigationTitle("Macros")
        }
    }

    private func describe(_ macro: MacroDef) -> String {
        let mask = macro.steps.first?.buttonsMask ?? 0
        let buttons = SwitchButton.allCases.filter { mask & $0.bit != 0 }.map(\.label)
        return "→ \(buttons.joined(separator: " + "))"
    }

    private func chordBinding(for input: PhysicalInput) -> Binding<Bool> {
        Binding(
            get: { chord.contains(input) },
            set: { on in if on { chord.insert(input) } else { chord.remove(input) } })
    }

    private func add() {
        guard var config = client.config else { return }
        config.macros.append(MacroDef.chord(Array(chord), to: output))
        chord.removeAll()
        Task { await client.apply(config) }
    }

    private func remove(at index: Int) {
        guard var config = client.config, config.macros.indices.contains(index) else { return }
        config.macros.remove(at: index)
        Task { await client.apply(config) }
    }
}
