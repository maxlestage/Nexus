import SwiftUI

@main
struct NexusOneApp: App {
    @StateObject private var transport = Transport()

    var body: some Scene {
        WindowGroup {
            RootView().environmentObject(transport)
        }
    }
}
