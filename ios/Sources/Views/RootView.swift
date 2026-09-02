import SwiftUI

struct RootView: View {
    @EnvironmentObject private var client: ControllerClient

    var body: some View {
        Group {
            if client.state.isReady, client.config != nil {
                TabView {
                    MappingView().tabItem { Label("Boutons", systemImage: "square.grid.2x2") }
                    TurboView().tabItem { Label("Turbo", systemImage: "bolt") }
                    MacrosView().tabItem { Label("Macros", systemImage: "wand.and.stars") }
                    StatsView().tabItem { Label("Stats", systemImage: "chart.bar") }
                    SettingsView().tabItem { Label("Réglages", systemImage: "gearshape") }
                }
            } else {
                ConnectView()
            }
        }
        .overlay(alignment: .top) { OTABanner() }
        .animation(.default, value: client.state)
    }
}

/// Écran d'accueil tant que la manette n'est pas jointe.
struct ConnectView: View {
    @EnvironmentObject private var client: ControllerClient

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            Image(systemName: "gamecontroller")
                .font(.system(size: 68, weight: .light))
                .foregroundStyle(.tint)

            VStack(spacing: 8) {
                Text("Nexus One").font(.largeTitle.bold())
                Text(message)
                    .font(.body)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 32)
            }

            if case .unavailable = client.state {
                EmptyView()
            } else if client.state == .scanning || client.state == .connecting {
                ProgressView().controlSize(.large)
            } else {
                Button("Rechercher la manette") { client.startScan() }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
            }

            if let error = client.lastError {
                Text(error)
                    .font(.footnote)
                    .foregroundStyle(.red)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 32)
            }
            Spacer()
        }
    }

    private var message: String {
        switch client.state {
        case .idle: return "Allumez la manette, puis lancez la recherche."
        case .scanning: return "Recherche de la manette…"
        case .connecting: return "Connexion en cours…"
        case .ready: return "Connectée."
        case .unavailable(let why): return why
        }
    }
}

/// Bandeau de progression affiché pendant une mise à jour du firmware.
struct OTABanner: View {
    @EnvironmentObject private var client: ControllerClient

    var body: some View {
        if let progress = client.otaProgress {
            VStack(spacing: 6) {
                Text("Mise à jour du firmware — \(progress) %").font(.footnote.weight(.semibold))
                ProgressView(value: Double(progress), total: 100)
                Text("Ne coupez pas la manette.")
                    .font(.caption2).foregroundStyle(.secondary)
            }
            .padding(12)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
            .padding(.horizontal, 16)
            .transition(.move(edge: .top).combined(with: .opacity))
        }
    }
}
