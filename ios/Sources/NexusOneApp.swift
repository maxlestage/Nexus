import SwiftUI

@main
struct NexusOneApp: App {
    @StateObject private var client = ControllerClient()

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(client)
        }
    }
}
