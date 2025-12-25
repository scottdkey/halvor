import Foundation
import HalvorSwiftFFI

/// Swift wrapper for Halvor client functionality
/// This provides a more Swift-idiomatic API on top of the FFI bindings
public class HalvorClient {
    private let ffiClient: HalvorSwiftFFI.HalvorClient
    
    /// Initialize a new Halvor client
    /// - Parameter agentPort: Optional port for agent communication (default: 23500)
    public init(agentPort: UInt16? = nil) {
        self.ffiClient = HalvorSwiftFFI.HalvorClient(agentPort: agentPort ?? 0)
    }
    
    /// Discover all available agents on the network
    /// - Returns: Array of discovered hosts
    /// - Throws: HalvorError if discovery fails
    public func discoverAgents() throws -> [DiscoveredHost] {
        let ffiHosts = try ffiClient.discoverAgents()
        return ffiHosts.map { host in
            DiscoveredHost(
                hostname: host.hostname,
                localIP: host.local_ip.isEmpty ? nil : host.local_ip,
                tailscaleIP: host.tailscale_ip.isEmpty ? nil : host.tailscale_ip,
                tailscaleHostname: host.tailscale_hostname.isEmpty ? nil : host.tailscale_hostname,
                agentPort: host.agent_port,
                reachable: host.reachable
            )
        }
    }
    
    /// Discover agents via Tailscale
    /// - Returns: Array of discovered hosts
    /// - Throws: HalvorError if discovery fails
    public func discoverViaTailscale() throws -> [DiscoveredHost] {
        let ffiHosts = try ffiClient.discoverViaTailscale()
        return ffiHosts.map { host in
            DiscoveredHost(
                hostname: host.hostname,
                localIP: host.local_ip.isEmpty ? nil : host.local_ip,
                tailscaleIP: host.tailscale_ip.isEmpty ? nil : host.tailscale_ip,
                tailscaleHostname: host.tailscale_hostname.isEmpty ? nil : host.tailscale_hostname,
                agentPort: host.agent_port,
                reachable: host.reachable
            )
        }
    }
    
    /// Discover agents on local network
    /// - Returns: Array of discovered hosts
    /// - Throws: HalvorError if discovery fails
    public func discoverViaLocalNetwork() throws -> [DiscoveredHost] {
        let ffiHosts = try ffiClient.discoverViaLocalNetwork()
        return ffiHosts.map { host in
            DiscoveredHost(
                hostname: host.hostname,
                localIP: host.local_ip.isEmpty ? nil : host.local_ip,
                tailscaleIP: host.tailscale_ip.isEmpty ? nil : host.tailscale_ip,
                tailscaleHostname: host.tailscale_hostname.isEmpty ? nil : host.tailscale_hostname,
                agentPort: host.agent_port,
                reachable: host.reachable
            )
        }
    }
    
    /// Ping an agent to check if it's reachable
    /// - Parameters:
    ///   - host: Host address (IP or hostname)
    ///   - port: Agent port (default: 23500)
    /// - Returns: true if agent is reachable
    /// - Throws: HalvorError if ping fails
    public func pingAgent(host: String, port: UInt16 = 23500) throws -> Bool {
        return ffiClient.pingAgent(host: host, port: port)
    }
    
    /// Get host information from an agent
    /// - Parameters:
    ///   - host: Host address (IP or hostname)
    ///   - port: Agent port (default: 23500)
    /// - Returns: Host information
    /// - Throws: HalvorError if request fails
    public func getHostInfo(host: String, port: UInt16 = 23500) throws -> HostInfo {
        let ffiInfo = try ffiClient.getHostInfo(host: host, port: port)
        return HostInfo(
            dockerVersion: ffiInfo.docker_version.isEmpty ? nil : ffiInfo.docker_version,
            tailscaleInstalled: ffiInfo.tailscale_installed,
            portainerInstalled: ffiInfo.portainer_installed
        )
    }
    
    /// Execute a command on a remote agent
    /// - Parameters:
    ///   - host: Host address (IP or hostname)
    ///   - port: Agent port (default: 23500)
    ///   - command: Command to execute
    ///   - args: Command arguments
    /// - Returns: Command output
    /// - Throws: HalvorError if execution fails
    public func executeCommand(
        host: String,
        port: UInt16 = 23500,
        command: String,
        args: [String] = []
    ) throws -> String {
        return try ffiClient.executeCommand(host: host, port: port, command: command, args: args)
    }
}

// HalvorClient is used across concurrent tasks; the underlying FFI client is thread-safe.
extension HalvorClient: @unchecked Sendable {}

/// Swift wrapper for discovered host information
public struct DiscoveredHost: Sendable {
    public let hostname: String
    public let localIP: String?
    public let tailscaleIP: String?
    public let tailscaleHostname: String?
    public let agentPort: UInt16
    public let reachable: Bool
    
    init(
        hostname: String,
        localIP: String?,
        tailscaleIP: String?,
        tailscaleHostname: String?,
        agentPort: UInt16,
        reachable: Bool
    ) {
        self.hostname = hostname
        self.localIP = localIP
        self.tailscaleIP = tailscaleIP
        self.tailscaleHostname = tailscaleHostname
        self.agentPort = agentPort
        self.reachable = reachable
    }
    
    /// Primary IP address (prefers Tailscale IP if available)
    public var primaryIP: String? {
        return tailscaleIP ?? localIP
    }
    
    /// Display name for the host
    public var displayName: String {
        return tailscaleHostname ?? hostname
    }
}

/// Swift wrapper for host information
public struct HostInfo: Sendable {
    public let dockerVersion: String?
    public let tailscaleInstalled: Bool
    public let portainerInstalled: Bool
    
    init(
        dockerVersion: String?,
        tailscaleInstalled: Bool,
        portainerInstalled: Bool
    ) {
        self.dockerVersion = dockerVersion
        self.tailscaleInstalled = tailscaleInstalled
        self.portainerInstalled = portainerInstalled
    }
}

/// Error types for Halvor operations
public enum HalvorError: Error {
    case connectionFailed(String)
    case agentError(String)
    case discoveryError(String)
    case unknown(String)
    
    init(_ message: String) {
        // Parse error message to determine error type
        if message.contains("Connection failed") || message.contains("connection") {
            self = .connectionFailed(message)
        } else if message.contains("Agent error") {
            self = .agentError(message)
        } else if message.contains("discover") || message.contains("Discovery") {
            self = .discoveryError(message)
        } else {
            self = .unknown(message)
        }
    }
}
