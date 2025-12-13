import SwiftUI
import HalvorSwift

struct ContentView: View {
    @StateObject private var viewModel = AgentDiscoveryViewModel()
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                if viewModel.isDiscovering {
                    ProgressView("Discovering agents...")
                        .padding()
                } else if let error = viewModel.error {
                    VStack {
                        Image(systemName: "exclamationmark.triangle")
                            .font(.largeTitle)
                            .foregroundColor(.red)
                        Text("Error")
                            .font(.headline)
                        Text(error)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding()
                    }
                    .padding()
                } else if viewModel.hosts.isEmpty {
                    VStack {
                        Image(systemName: "network")
                            .font(.largeTitle)
                            .foregroundColor(.secondary)
                        Text("No agents discovered")
                            .font(.headline)
                        Text("Click 'Discover Agents' to search for Halvor agents on your network")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                            .padding()
                    }
                    .padding()
                } else {
                    List(viewModel.hosts) { host in
                        HostRowView(host: host)
                    }
                }
                
                Button(action: {
                    viewModel.discoverAgents()
                }) {
                    HStack {
                        Image(systemName: "magnifyingglass")
                        Text("Discover Agents")
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.accentColor)
                    .foregroundColor(.white)
                    .cornerRadius(10)
                }
                .disabled(viewModel.isDiscovering)
                .padding()
            }
            .navigationTitle("Halvor Agents")
            .refreshable {
                viewModel.discoverAgents()
            }
        }
    }
}

struct HostRowView: View {
    let host: DiscoveredHost
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(host.displayName)
                    .font(.headline)
                Spacer()
                if host.reachable {
                    Image(systemName: "checkmark.circle.fill")
                        .foregroundColor(.green)
                } else {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundColor(.red)
                }
            }
            if let ip = host.primaryIP {
                Text(ip)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
}


extension DiscoveredHost: Identifiable{
    
    public var id: String {
        hostname + (primaryIP ?? "")
    }
}

@MainActor
class AgentDiscoveryViewModel: ObservableObject {
    @Published var hosts: [DiscoveredHost] = []
    @Published var isDiscovering = false
    @Published var error: String?
    
    private let client = HalvorClient()
    
    func discoverAgents() {
        isDiscovering = true
        error = nil
        
        Task {
            do {
                let discovered = try client.discoverAgents()
                self.hosts = discovered
                self.isDiscovering = false
            } catch {
                self.error = error.localizedDescription
                self.isDiscovering = false
            }
        }
    }
}
