import SwiftUI

struct SettingsView: View {
    @EnvironmentObject private var client: ControllerClient
    @State private var ssid = ""
    @State private var password = ""
    @State private var firmwareURL = ""
    @State private var confirmReset = false
    @State private var confirmOTA = false

    var body: some View {
        NavigationStack {
            Form {
                if let config = client.config {
                    Section("Éclairage") {
                        Picker("Mode", selection: ledModeBinding) {
                            ForEach(LedMode.allCases) { Text($0.label).tag($0) }
                        }
                        ColorPicker("Couleur", selection: colorBinding, supportsOpacity: false)
                        VStack(alignment: .leading) {
                            Text("Luminosité : \(config.leds.brightness)")
                            Slider(value: brightnessBinding, in: 0...255, step: 1)
                        }
                    }

                    Section("Vibrations") {
                        Toggle("Activées", isOn: hapticsEnabledBinding)
                        VStack(alignment: .leading) {
                            Text("Force : \(config.haptics.strength)")
                            Slider(value: strengthBinding, in: 0...127, step: 1)
                        }
                        Toggle("Clic à chaque appui", isOn: clickBinding)
                        Button("Tester la vibration") { Task { await client.testHaptic() } }
                    }
                }

                Section("Manette") {
                    if let firmware = client.firmwareVersion {
                        LabeledContent("Firmware", value: firmware)
                    }
                    if let battery = client.battery {
                        LabeledContent("Batterie") {
                            Text("\(battery.percent) % · \(battery.volts)\(battery.charging ? " ⚡" : "")")
                        }
                    }
                    Button("Actualiser la batterie") { Task { await client.refreshBattery() } }
                    Button("Identifier la manette") { Task { await client.identify() } }
                    Button("Se déconnecter") { client.disconnect() }
                }

                Section {
                    TextField("Réseau WiFi (SSID)", text: $ssid)
                        .textInputAutocapitalization(.never).autocorrectionDisabled()
                    SecureField("Mot de passe", text: $password)
                    TextField("URL du firmware", text: $firmwareURL)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .keyboardType(.URL)
                    Button("Lancer la mise à jour") { confirmOTA = true }
                        .disabled(ssid.isEmpty || firmwareURL.isEmpty)
                } header: {
                    Text("Mise à jour du firmware")
                } footer: {
                    Text("La manette rejoint votre WiFi et télécharge le firmware, puis redémarre.")
                }

                Section {
                    Button("Réglages d'usine", role: .destructive) { confirmReset = true }
                }

                SaveSection()
            }
            .navigationTitle("Réglages")
            .confirmationDialog(
                "Restaurer les réglages d'usine ?", isPresented: $confirmReset,
                titleVisibility: .visible
            ) {
                Button("Restaurer", role: .destructive) { Task { await client.factoryReset() } }
                Button("Annuler", role: .cancel) {}
            } message: {
                Text("Le remappage, le turbo et les macros seront perdus.")
            }
            .confirmationDialog(
                "Mettre à jour le firmware ?", isPresented: $confirmOTA, titleVisibility: .visible
            ) {
                Button("Mettre à jour") {
                    Task { await client.startOTA(ssid: ssid, password: password, url: firmwareURL) }
                }
                Button("Annuler", role: .cancel) {}
            } message: {
                Text("Ne coupez pas la manette pendant la mise à jour.")
            }
            .alert(
                "Erreur", isPresented: .constant(client.lastError != nil),
                actions: { Button("OK") { client.lastError = nil } },
                message: { Text(client.lastError ?? "") })
        }
    }

    // MARK: Liaisons

    private func configBinding<T>(
        _ get: @escaping (ControllerConfig) -> T,
        _ set: @escaping (inout ControllerConfig, T) -> Void,
        default fallback: T
    ) -> Binding<T> {
        Binding(
            get: { client.config.map(get) ?? fallback },
            set: { newValue in
                guard var config = client.config else { return }
                set(&config, newValue)
                Task { await client.apply(config) }
            })
    }

    private var ledModeBinding: Binding<LedMode> {
        configBinding({ $0.leds.mode }, { $0.leds.mode = $1 }, default: .breathe)
    }
    private var brightnessBinding: Binding<Double> {
        configBinding(
            { Double($0.leds.brightness) }, { $0.leds.brightness = UInt8($1) }, default: 80)
    }
    private var hapticsEnabledBinding: Binding<Bool> {
        configBinding({ $0.haptics.enabled }, { $0.haptics.enabled = $1 }, default: true)
    }
    private var strengthBinding: Binding<Double> {
        configBinding(
            { Double($0.haptics.strength) }, { $0.haptics.strength = UInt8($1) }, default: 90)
    }
    private var clickBinding: Binding<Bool> {
        configBinding({ $0.haptics.clickOnPress }, { $0.haptics.clickOnPress = $1 }, default: false)
    }

    private var colorBinding: Binding<Color> {
        Binding(
            get: {
                guard let leds = client.config?.leds else { return .blue }
                return Color(
                    red: Double(leds.r) / 255, green: Double(leds.g) / 255,
                    blue: Double(leds.b) / 255)
            },
            set: { newColor in
                guard var config = client.config else { return }
                let components = UIColor(newColor).rgb
                config.leds.r = components.r
                config.leds.g = components.g
                config.leds.b = components.b
                Task { await client.apply(config) }
            })
    }
}

extension UIColor {
    /// Composantes 8 bits, bornées : `getRed` peut sortir de 0...1 dans les
    /// espaces colorimétriques étendus de l'iPhone.
    var rgb: (r: UInt8, g: UInt8, b: UInt8) {
        var red: CGFloat = 0, green: CGFloat = 0, blue: CGFloat = 0, alpha: CGFloat = 0
        getRed(&red, green: &green, blue: &blue, alpha: &alpha)
        let clamp = { (v: CGFloat) in UInt8(max(0, min(1, v)) * 255) }
        return (clamp(red), clamp(green), clamp(blue))
    }
}
