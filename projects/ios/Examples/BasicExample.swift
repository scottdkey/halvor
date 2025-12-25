import Foundation
import HalvorSwift

/// Basic example demonstrating HalvorSwift usage
@main
struct BasicExample {
    static func main() async {
        let client = HalvorClient()
        
        print("Halvor Swift Example")
        print("===================\n")
        
        // Discover agents
        print("Discovering agents...")
        do {
            let hosts = try client.discoverAgents()
            
            if hosts.isEmpty {
                print("No agents discovered.")
                print("\nMake sure:")
                print("  - Agents are running on other hosts (halvor agent start)")
                print("  - Tailscale is configured and devices are connected")
                print("  - Firewall allows connections on port 23500")
            } else {
                print("Discovered \(hosts.count) agent(s):\n")
                
                for host in hosts {
                    print("Host: \(host.displayName)")
                    if let ip = host.primaryIP {
                        print("  IP: \(ip)")
                    }
                    print("  Reachable: \(host.reachable ? "Yes" : "No")")
                    
                    // Get host info if reachable
                    if host.reachable, let ip = host.primaryIP {
                        do {
                            let info = try client.getHostInfo(host: ip, port: host.agentPort)
                            print("  Docker Version: \(info.dockerVersion ?? "unknown")")
                            print("  Tailscale Installed: \(info.tailscaleInstalled ? "Yes" : "No")")
                            print("  Portainer Installed: \(info.portainerInstalled ? "Yes" : "No")")
                        } catch {
                            print("  Error getting host info: \(error)")
                        }
                    }
                    print()
                }
            }
        } catch HalvorError.discoveryError(let message) {
            print("Discovery error: \(message)")
        } catch {
            print("Error: \(error)")
        }
        
        // Example: Ping a specific agent
        print("\nPinging localhost agent...")
        do {
            let isReachable = try client.pingAgent(host: "127.0.0.1", port: 23500)
            print("Agent is \(isReachable ? "reachable" : "not reachable")")
        } catch {
            print("Ping failed: \(error)")
        }
    }
}
