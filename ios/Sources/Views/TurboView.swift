import SwiftUI

struct TurboView: View {
    @EnvironmentObject private var client: ControllerClient

    var body: some View {
        NavigationStack {
            Form {
                if let config = client.config {
                    Section {
                        Stepper(
                            "Cadence : \(config.turbo.rateHz) appuis/s",
                            value: rateBinding, in: 1...30)
                    } footer: {
                        Text("Sur la manette : maintenez TURBO et appuyez sur un bouton pour activer sa rafale sans passer par l'application.")
                    }

                    Section("Boutons en rafale") {
                        ForEach(SwitchButton.allCases) { button in
                            Toggle(button.label, isOn: turboBinding(for: button))
                        }
                    }
                }
                SaveSection()
            }
            .navigationTitle("Turbo")
        }
    }

    private var rateBinding: Binding<UInt8> {
        Binding(
            get: { client.config?.turbo.rateHz ?? 12 },
            set: { newValue in
                guard var config = client.config else { return }
                config.turbo.rateHz = newValue
                Task { await client.apply(config) }
            })
    }

    private func turboBinding(for button: SwitchButton) -> Binding<Bool> {
        Binding(
            get: { (client.config?.turbo.enabledMask ?? 0) & button.bit != 0 },
            set: { _ in
                guard var config = client.config else { return }
                config.turbo.enabledMask ^= button.bit
                Task { await client.apply(config) }
            })
    }
}
