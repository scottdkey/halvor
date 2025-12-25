import Foundation

// C FFI bindings for Halvor
@_silgen_name("halvor_client_new")
func halvor_client_new(_ agent_port: UInt16) -> UnsafeMutableRawPointer?

@_silgen_name("halvor_client_free")
func halvor_client_free(_ ptr: UnsafeMutableRawPointer)

@_silgen_name("halvor_client_discover_agents")
func halvor_client_discover_agents(_ ptr: UnsafeMutableRawPointer) -> UnsafeMutablePointer<CChar>?

@_silgen_name("halvor_client_discover_via_tailscale")
func halvor_client_discover_via_tailscale(_ ptr: UnsafeMutableRawPointer) -> UnsafeMutablePointer<CChar>?

@_silgen_name("halvor_client_discover_via_local_network")
func halvor_client_discover_via_local_network(_ ptr: UnsafeMutableRawPointer) -> UnsafeMutablePointer<CChar>?

@_silgen_name("halvor_client_ping_agent")
func halvor_client_ping_agent(_ ptr: UnsafeMutableRawPointer, _ host: UnsafePointer<CChar>, _ port: UInt16) -> Int32

@_silgen_name("halvor_client_get_host_info")
func halvor_client_get_host_info(_ ptr: UnsafeMutableRawPointer, _ host: UnsafePointer<CChar>, _ port: UInt16) -> UnsafeMutablePointer<CChar>?

@_silgen_name("halvor_client_execute_command")
func halvor_client_execute_command(_ ptr: UnsafeMutableRawPointer, _ host: UnsafePointer<CChar>, _ port: UInt16, _ command: UnsafePointer<CChar>, _ args_json: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>?

@_silgen_name("halvor_string_free")
func halvor_string_free(_ ptr: UnsafeMutablePointer<CChar>)

// Swift wrapper types
public struct DiscoveredHost: Codable {
    public let hostname: String
    public let local_ip: String
    public let tailscale_ip: String
    public let tailscale_hostname: String
    public let agent_port: UInt16
    public let reachable: Bool
}

public struct HostInfo: Codable {
    public let docker_version: String
    public let tailscale_installed: Bool
    public let portainer_installed: Bool
}

// Swift wrapper for HalvorClient
public class HalvorClient {
    private let ptr: UnsafeMutableRawPointer
    
    public init(agentPort: UInt16? = nil) {
        let port = agentPort ?? 0
        guard let clientPtr = halvor_client_new(port) else {
            fatalError("Failed to create HalvorClient")
        }
        self.ptr = clientPtr
    }
    
    deinit {
        halvor_client_free(ptr)
    }
    
    private func getString(from cString: UnsafeMutablePointer<CChar>?) -> String? {
        guard let cString = cString else { return nil }
        defer { halvor_string_free(cString) }
        return String(cString: cString)
    }
    
    public func discoverAgents() throws -> [DiscoveredHost] {
        guard let jsonPtr = halvor_client_discover_agents(ptr),
              let json = getString(from: jsonPtr),
              let data = json.data(using: .utf8) else {
            throw HalvorError.unknown("Failed to discover agents")
        }
        return try JSONDecoder().decode([DiscoveredHost].self, from: data)
    }
    
    public func discoverViaTailscale() throws -> [DiscoveredHost] {
        guard let jsonPtr = halvor_client_discover_via_tailscale(ptr),
              let json = getString(from: jsonPtr),
              let data = json.data(using: .utf8) else {
            throw HalvorError.unknown("Failed to discover agents via Tailscale")
        }
        return try JSONDecoder().decode([DiscoveredHost].self, from: data)
    }
    
    public func discoverViaLocalNetwork() throws -> [DiscoveredHost] {
        guard let jsonPtr = halvor_client_discover_via_local_network(ptr),
              let json = getString(from: jsonPtr),
              let data = json.data(using: .utf8) else {
            throw HalvorError.unknown("Failed to discover agents on local network")
        }
        return try JSONDecoder().decode([DiscoveredHost].self, from: data)
    }
    
    public func pingAgent(host: String, port: UInt16) -> Bool {
        let result = host.withCString { hostPtr in
            halvor_client_ping_agent(ptr, hostPtr, port)
        }
        return result != 0
    }
    
    public func getHostInfo(host: String, port: UInt16) throws -> HostInfo {
        guard let jsonPtr = host.withCString({ hostPtr in
            halvor_client_get_host_info(ptr, hostPtr, port)
        }),
        let json = getString(from: jsonPtr),
        let data = json.data(using: .utf8) else {
            throw HalvorError.unknown("Failed to get host info")
        }
        return try JSONDecoder().decode(HostInfo.self, from: data)
    }
    
    public func executeCommand(host: String, port: UInt16, command: String, args: [String] = []) throws -> String {
        let argsJson = try JSONEncoder().encode(args)
        let argsJsonString = String(data: argsJson, encoding: .utf8) ?? "[]"
        
        guard let jsonPtr = host.withCString({ hostPtr in
            command.withCString { cmdPtr in
                argsJsonString.withCString { argsPtr in
                    halvor_client_execute_command(ptr, hostPtr, port, cmdPtr, argsPtr)
                }
            }
        }),
        let json = getString(from: jsonPtr),
        let data = json.data(using: .utf8) else {
            throw HalvorError.unknown("Failed to execute command")
        }
        return try JSONDecoder().decode(String.self, from: data)
    }
}

public enum HalvorError: Error {
    case unknown(String)
}
