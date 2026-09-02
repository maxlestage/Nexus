import SwiftUI

struct StatsView: View {
    @EnvironmentObject private var client: ControllerClient

    var body: some View {
        NavigationStack {
            List {
                if let stats = client.stats {
                    Section {
                        LabeledContent("Temps de jeu", value: formatted(stats.uptimeS))
                        LabeledContent("Macros déclenchées", value: "\(stats.macrosFired)")
                    }

                    Section("Appuis par bouton") {
                        let maximum = max(stats.presses.max() ?? 1, 1)
                        ForEach(PhysicalInput.allCases) { input in
                            let count = input.rawValue < stats.presses.count
                                ? stats.presses[input.rawValue] : 0
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    Text(input.label)
                                    Spacer()
                                    Text("\(count)").monospacedDigit().foregroundStyle(.secondary)
                                }
                                ProgressView(value: Double(count), total: Double(maximum))
                            }
                        }
                    }

                    Section {
                        Button("Remettre les compteurs à zéro", role: .destructive) {
                            Task { await client.resetStats() }
                        }
                    }
                } else {
                    Text("Aucune statistique chargée.").foregroundStyle(.secondary)
                }
            }
            .navigationTitle("Statistiques")
            .refreshable { await client.refreshStats() }
            .task { await client.refreshStats() }
        }
    }

    private func formatted(_ seconds: UInt32) -> String {
        let h = seconds / 3600, m = (seconds % 3600) / 60
        return h > 0 ? "\(h) h \(String(format: "%02d", m)) min" : "\(m) min"
    }
}
