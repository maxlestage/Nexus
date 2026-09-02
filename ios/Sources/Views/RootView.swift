import SwiftUI

/// Rend le modèle de vue produit par Rust. Ne contient aucun libellé ni
/// aucune règle métier : uniquement la traduction en composants SwiftUI.
struct RootView: View {
    @EnvironmentObject private var transport: Transport
    @State private var selectedTab = 0

    var body: some View {
        Group {
            if let model = transport.model {
                content(model)
                    .overlay(alignment: .top) { banner(model.banner) }
                    .alert(
                        "Erreur",
                        isPresented: Binding(
                            get: { model.error != nil },
                            set: { if !$0 { transport.send("error.dismiss") } }),
                        actions: { Button("OK") { transport.send("error.dismiss") } },
                        message: { Text(model.error ?? "") })
            } else {
                ProgressView()
            }
        }
    }

    @ViewBuilder
    private func content(_ model: ViewModel) -> some View {
        switch model.screen {
        case .connect(let title, let message, let action, let spinner):
            ConnectScreen(title: title, message: message, action: action, spinner: spinner)
        case .tabs(let tabs):
            TabView(selection: $selectedTab) {
                ForEach(Array(tabs.enumerated()), id: \.element.id) { index, tab in
                    NavigationStack {
                        Form {
                            ForEach(tab.sections) { section in
                                SectionView(section: section)
                            }
                        }
                        .navigationTitle(tab.title)
                    }
                    .tabItem { Label(tab.title, systemImage: tab.icon) }
                    .tag(index)
                }
            }
            .onChange(of: selectedTab) { _, new in transport.send("tab", new) }
        }
    }

    @ViewBuilder
    private func banner(_ banner: Banner?) -> some View {
        if case .ota(let percent, let title, let message) = banner {
            VStack(spacing: 6) {
                Text(title).font(.footnote.weight(.semibold))
                ProgressView(value: Double(percent), total: 100)
                Text(message).font(.caption2).foregroundStyle(.secondary)
            }
            .padding(12)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
            .padding(.horizontal, 16)
            .transition(.move(edge: .top).combined(with: .opacity))
        }
    }
}

struct ConnectScreen: View {
    @EnvironmentObject private var transport: Transport
    let title: String
    let message: String
    let action: Row?
    let spinner: Bool

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            Image(systemName: "gamecontroller")
                .font(.system(size: 68, weight: .light))
                .foregroundStyle(.tint)
            VStack(spacing: 8) {
                Text(title).font(.largeTitle.bold())
                Text(message)
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 32)
            }
            if spinner {
                ProgressView().controlSize(.large)
            } else if let action {
                RowView(row: action)
                    .buttonStyle(.borderedProminent)
                    .controlSize(.large)
            }
            Spacer()
        }
    }
}

struct SectionView: View {
    let section: Section

    var body: some View {
        SwiftUI.Section {
            ForEach(section.rows) { row in RowView(row: row) }
        } header: {
            if let header = section.header { Text(header) }
        } footer: {
            if let footer = section.footer { Text(footer) }
        }
    }
}
