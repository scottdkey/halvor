import Foundation
import HalvorSwift

/// Example demonstrating async/await usage with HalvorSwift
@main
struct AsyncExample {
    static func main() async {
        let client = HalvorClient()
        
        print("Halvor Swift Async Example")
        print("=========================\n")
        
        // Discover agents asynchronously
        await withTaskGroup(of: Void.self) { group in
            group.addTask {
                do {
                    print("Discovering via Tailscale...")
                    let hosts = try client.discoverViaTailscale()
                    print("Found \(hosts.count) agent(s) via Tailscale")
                    for host in hosts {
                        print("  - \(host.displayName) at \(host.primaryIP ?? "unknown")")
                    }
                } catch {
                    print("Tailscale discovery error: \(error)")
                }
            }
            
            group.addTask {
                do {
                    print("Discovering via local network...")
                    let hosts = try client.discoverViaLocalNetwork()
                    print("Found \(hosts.count) agent(s) on local network")
                    for host in hosts {
                        print("  - \(host.displayName) at \(host.primaryIP ?? "unknown")")
                    }
                } catch {
                    print("Local network discovery error: \(error)")
                }
            }
        }
        
        // Get host info for all discovered agents
        print("\nFetching host information...")
        do {
            let hosts = try client.discoverAgents()
            
            await withTaskGroup(of: (String, Result<HostInfo, Error>).self) { group in
                for host in hosts where host.reachable {
                    guard let ip = host.primaryIP else { continue }
                    
                    group.addTask {
                        let result = Result {
                            try client.getHostInfo(host: ip, port: host.agentPort)
                        }
                        return (host.displayName, result)
                    }
                }
                
                for await (hostname, result) in group {
                    switch result {
                    case .success(let info):
                        print("\(hostname):")
                        print("  Docker: \(info.dockerVersion ?? "unknown")")
                        print("  Portainer: \(info.portainerInstalled ? "installed" : "not installed")")
                    case .failure(let error):
                        print("\(hostname): Error - \(error)")
                    }
                }
            }
        } catch {
            print("Error discovering agents: \(error)")
        }
    }
}
