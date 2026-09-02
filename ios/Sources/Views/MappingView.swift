import SwiftUI

/// Remappage des entrées physiques, couche normale et couche SHIFT.
struct MappingView: View {
    @EnvironmentObject private var client: ControllerClient
    @State private var showShiftLayer = false

    private var remappable: [PhysicalInput] {
        PhysicalInput.allCases.filter { !$0.isModifier }
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Picker("Couche", selection: $showShiftLayer) {
                        Text("Normale").tag(false)
                        Text("SHIFT").tag(true)
                    }
                    .pickerStyle(.segmented)
                } footer: {
                    Text(showShiftLayer
                         ? "Ce que font les boutons tant que le modificateur SHIFT est maintenu."
                         : "Ce que font les boutons en temps normal.")
                }

                if let config = client.config {
                    Section("Boutons") {
                        ForEach(remappable) { input in
                            Picker(input.label, selection: binding(for: input)) {
                                Text("— aucun —").tag(SwitchButton?.none)
                                ForEach(SwitchButton.allCases) { button in
                                    Text(button.label).tag(SwitchButton?.some(button))
                                }
                            }
                        }
                    }

                    Section("Joystick") {
                        Picker("Envoie vers", selection: stickBinding) {
                            Text("Stick gauche").tag(StickTarget.left)
                            Text("Stick droit").tag(StickTarget.right)
                        }
                        VStack(alignment: .leading) {
                            Text("Zone morte : \(config.stickDeadzone) ‰")
                            Slider(
                                value: deadzoneBinding, in: 0...400, step: 10)
                        }
                    }
                }

                SaveSection()
            }
            .navigationTitle("Boutons")
        }
    }

    private func binding(for input: PhysicalInput) -> Binding<SwitchButton?> {
        Binding(
            get: { client.config?.mapping(input, shift: showShiftLayer) },
            set: { newValue in
                guard var config = client.config else { return }
                config.setMapping(input, shift: showShiftLayer, to: newValue)
                Task { await client.apply(config) }
            })
    }

    private var stickBinding: Binding<StickTarget> {
        Binding(
            get: {
                guard let c = client.config else { return .left }
                return showShiftLayer ? c.stickShift : c.stickNormal
            },
            set: { newValue in
                guard var config = client.config else { return }
                if showShiftLayer { config.stickShift = newValue } else { config.stickNormal = newValue }
                Task { await client.apply(config) }
            })
    }

    private var deadzoneBinding: Binding<Double> {
        Binding(
            get: { Double(client.config?.stickDeadzone ?? 80) },
            set: { newValue in
                guard var config = client.config else { return }
                config.stickDeadzone = UInt16(newValue)
                Task { await client.apply(config) }
            })
    }
}

/// Rappel commun : les changements sont immédiats, l'enregistrement les rend
/// permanents.
struct SaveSection: View {
    @EnvironmentObject private var client: ControllerClient

    var body: some View {
        Section {
            Button("Enregistrer sur la manette") { Task { await client.save() } }
        } footer: {
            Text("Les changements s'appliquent tout de suite. L'enregistrement les conserve après extinction.")
        }
    }
}
