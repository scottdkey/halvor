//! Client library code generation for TypeScript, Kotlin, and Swift
//! Generates typed client libraries from API definitions

use super::api_def::{ApiDefinition, ApiEndpoint, HttpMethod};
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};

/// Generate all client libraries
pub fn generate_all_clients(output_base: &PathBuf) -> Result<()> {
    let endpoints = ApiDefinition::endpoints();
    
    generate_typescript_client(&endpoints, output_base)?;
    generate_kotlin_client(&endpoints, output_base)?;
    generate_swift_client(&endpoints, output_base)?;
    
    Ok(())
}

/// Generate TypeScript client library
fn generate_typescript_client(endpoints: &[ApiEndpoint], output_base: &PathBuf) -> Result<()> {
    let output_dir = output_base.join("projects/web/src/lib/halvor-api");
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create TypeScript output directory: {:?}", output_dir))?;
    
    let mut code = String::from("// Auto-generated TypeScript API client\n");
    code.push_str("// DO NOT EDIT - This file is generated automatically\n\n");
    
    // Types
    code.push_str("export interface ApiResponse<T> {\n");
    code.push_str("  success: boolean;\n");
    code.push_str("  data?: T;\n");
    code.push_str("  error?: string;\n");
    code.push_str("}\n\n");
    
    code.push_str("export interface DiscoveredHost {\n");
    code.push_str("  hostname: string;\n");
    code.push_str("  localIp?: string;\n");
    code.push_str("  tailscaleIp?: string;\n");
    code.push_str("  tailscaleHostname?: string;\n");
    code.push_str("  agentPort: number;\n");
    code.push_str("  reachable: boolean;\n");
    code.push_str("}\n\n");
    
    code.push_str("export interface HostInfo {\n");
    code.push_str("  dockerVersion?: string;\n");
    code.push_str("  tailscaleInstalled: boolean;\n");
    code.push_str("  portainerInstalled: boolean;\n");
    code.push_str("}\n\n");
    
    code.push_str("export interface PingAgentRequest {\n");
    code.push_str("  host: string;\n");
    code.push_str("  port: number;\n");
    code.push_str("}\n\n");
    
    code.push_str("export interface GetHostInfoRequest {\n");
    code.push_str("  host: string;\n");
    code.push_str("  port: number;\n");
    code.push_str("}\n\n");
    
    code.push_str("export interface ExecuteCommandRequest {\n");
    code.push_str("  host: string;\n");
    code.push_str("  port: number;\n");
    code.push_str("  command: string;\n");
    code.push_str("  args: string[];\n");
    code.push_str("}\n\n");
    
    // Client class
    code.push_str("const API_BASE = import.meta.env.VITE_API_URL || '/api';\n\n");
    code.push_str("async function apiCall<T>(endpoint: string, options?: RequestInit): Promise<T> {\n");
    code.push_str("  const response = await fetch(`${API_BASE}${endpoint}`, {\n");
    code.push_str("    headers: {\n");
    code.push_str("      'Content-Type': 'application/json',\n");
    code.push_str("      ...options?.headers,\n");
    code.push_str("    },\n");
    code.push_str("    ...options,\n");
    code.push_str("  });\n\n");
    code.push_str("  if (!response.ok) {\n");
    code.push_str("    throw new Error(`API error: ${response.statusText}`);\n");
    code.push_str("  }\n\n");
    code.push_str("  const result: ApiResponse<T> = await response.json();\n\n");
    code.push_str("  if (!result.success) {\n");
    code.push_str("    throw new Error(result.error || 'Unknown API error');\n");
    code.push_str("  }\n\n");
    code.push_str("  return result.data!;\n");
    code.push_str("}\n\n");
    
    code.push_str("export class HalvorApiClient {\n");
    
    // Generate methods for each endpoint
    for endpoint in endpoints {
        let method_name = to_camel_case(&endpoint.handler);
        let return_type = map_rust_type_to_ts(&endpoint.response_type);
        
        code.push_str(&format!("  /** {}\n", endpoint.description));
        code.push_str(&format!("   * @returns Promise<{}>\n", return_type));
        code.push_str("   */\n");
        
        if let Some(ref req_type) = endpoint.request_type {
            code.push_str(&format!("  async {}(request: {}): Promise<{}> {{\n", method_name, req_type, return_type));
            code.push_str(&format!("    return apiCall<{}>('{}', {{\n", return_type, endpoint.path));
            code.push_str("      method: 'POST',\n");
            code.push_str("      body: JSON.stringify(request),\n");
            code.push_str("    });\n");
        } else {
            code.push_str(&format!("  async {}(): Promise<{}> {{\n", method_name, return_type));
            code.push_str(&format!("    return apiCall<{}>('{}');\n", return_type, endpoint.path));
        }
        code.push_str("  }\n\n");
    }
    
    code.push_str("}\n\n");
    code.push_str("// Default export\n");
    code.push_str("export const halvorApi = new HalvorApiClient();\n");
    
    fs::write(output_dir.join("client.ts"), code)
        .with_context(|| "Failed to write TypeScript client")?;
    
    Ok(())
}

/// Generate Kotlin client library
fn generate_kotlin_client(endpoints: &[ApiEndpoint], output_base: &PathBuf) -> Result<()> {
    let output_dir = output_base.join("projects/android/src/main/kotlin/dev/scottkey/halvor/api");
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create Kotlin output directory: {:?}", output_dir))?;
    
    let mut code = String::from("// Auto-generated Kotlin API client\n");
    code.push_str("// DO NOT EDIT - This file is generated automatically\n\n");
    code.push_str("package dev.scottkey.halvor.api\n\n");
    code.push_str("import kotlinx.serialization.Serializable\n");
    code.push_str("import kotlinx.serialization.json.Json\n");
    code.push_str("import kotlinx.coroutines.Dispatchers\n");
    code.push_str("import kotlinx.coroutines.withContext\n");
    code.push_str("import java.net.HttpURLConnection\n");
    code.push_str("import java.net.URL\n");
    code.push_str("import java.io.OutputStreamWriter\n");
    code.push_str("import java.io.BufferedReader\n");
    code.push_str("import java.io.InputStreamReader\n\n");
    
    // Types
    code.push_str("@Serializable\n");
    code.push_str("data class ApiResponse<T>(\n");
    code.push_str("    val success: Boolean,\n");
    code.push_str("    val data: T?,\n");
    code.push_str("    val error: String?\n");
    code.push_str(")\n\n");
    
    code.push_str("@Serializable\n");
    code.push_str("data class DiscoveredHost(\n");
    code.push_str("    val hostname: String,\n");
    code.push_str("    val localIp: String? = null,\n");
    code.push_str("    val tailscaleIp: String? = null,\n");
    code.push_str("    val tailscaleHostname: String? = null,\n");
    code.push_str("    val agentPort: Int,\n");
    code.push_str("    val reachable: Boolean\n");
    code.push_str(")\n\n");
    
    code.push_str("@Serializable\n");
    code.push_str("data class HostInfo(\n");
    code.push_str("    val dockerVersion: String? = null,\n");
    code.push_str("    val tailscaleInstalled: Boolean,\n");
    code.push_str("    val portainerInstalled: Boolean\n");
    code.push_str(")\n\n");
    
    code.push_str("@Serializable\n");
    code.push_str("data class PingAgentRequest(\n");
    code.push_str("    val host: String,\n");
    code.push_str("    val port: Int\n");
    code.push_str(")\n\n");
    
    code.push_str("@Serializable\n");
    code.push_str("data class GetHostInfoRequest(\n");
    code.push_str("    val host: String,\n");
    code.push_str("    val port: Int\n");
    code.push_str(")\n\n");
    
    code.push_str("@Serializable\n");
    code.push_str("data class ExecuteCommandRequest(\n");
    code.push_str("    val host: String,\n");
    code.push_str("    val port: Int,\n");
    code.push_str("    val command: String,\n");
    code.push_str("    val args: List<String>\n");
    code.push_str(")\n\n");
    
    // Client class
    code.push_str("class HalvorApiClient(private val baseUrl: String = \"http://localhost:8080\") {\n");
    code.push_str("    private val json = Json { ignoreUnknownKeys = true }\n\n");
    
    // Generate methods
    for endpoint in endpoints {
        let method_name = to_camel_case(&endpoint.handler);
        let return_type = map_rust_type_to_kotlin(&endpoint.response_type);
        
        code.push_str(&format!("    /** {}\n", endpoint.description));
        code.push_str(&format!("     * @return {}\n", return_type));
        code.push_str("     */\n");
        code.push_str("    suspend fun ");
        
        if let Some(ref req_type) = endpoint.request_type {
            code.push_str(&format!("{}(request: {}): {} = withContext(Dispatchers.IO) {{\n", method_name, req_type, return_type));
            code.push_str(&format!("        val url = URL(\"$baseUrl{}\")\n", endpoint.path));
            code.push_str("        val connection = url.openConnection() as HttpURLConnection\n");
            code.push_str("        connection.requestMethod = \"POST\"\n");
            code.push_str("        connection.setRequestProperty(\"Content-Type\", \"application/json\")\n");
            code.push_str("        connection.doOutput = true\n\n");
            code.push_str("        val requestBody = json.encodeToString(serializer(), request)\n");
            code.push_str("        OutputStreamWriter(connection.outputStream).use { writer ->\n");
            code.push_str("            writer.write(requestBody)\n");
            code.push_str("        }\n\n");
        } else {
            code.push_str(&format!("{}(): {} = withContext(Dispatchers.IO) {{\n", method_name, return_type));
            code.push_str(&format!("        val url = URL(\"$baseUrl{}\")\n", endpoint.path));
            code.push_str("        val connection = url.openConnection() as HttpURLConnection\n");
            code.push_str("        connection.requestMethod = \"GET\"\n");
        }
        
        code.push_str("        val responseCode = connection.responseCode\n");
        code.push_str("        if (responseCode != HttpURLConnection.HTTP_OK) {\n");
        code.push_str("            throw Exception(\"API error: $responseCode\")\n");
        code.push_str("        }\n\n");
        code.push_str(&format!("        val response: ApiResponse<{}> = json.decodeFromString(\n", return_type));
        code.push_str("            serializer(),\n");
        code.push_str("            BufferedReader(InputStreamReader(connection.inputStream)).use { it.readText() }\n");
        code.push_str("        )\n\n");
        code.push_str("        if (!response.success) {\n");
        code.push_str("            throw Exception(response.error ?: \"Unknown API error\")\n");
        code.push_str("        }\n\n");
        code.push_str("        response.data ?: throw Exception(\"No data in response\")\n");
        code.push_str("    }\n\n");
    }
    
    code.push_str("}\n");
    
    fs::write(output_dir.join("HalvorApiClient.kt"), code)
        .with_context(|| "Failed to write Kotlin client")?;
    
    Ok(())
}

/// Generate Swift client library
fn generate_swift_client(endpoints: &[ApiEndpoint], output_base: &PathBuf) -> Result<()> {
    let output_dir = output_base.join("projects/ios/Sources/HalvorApi");
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("Failed to create Swift output directory: {:?}", output_dir))?;
    
    let mut code = String::from("// Auto-generated Swift API client\n");
    code.push_str("// DO NOT EDIT - This file is generated automatically\n\n");
    code.push_str("import Foundation\n\n");
    
    // Types
    code.push_str("public struct ApiResponse<T: Codable>: Codable {\n");
    code.push_str("    public let success: Bool\n");
    code.push_str("    public let data: T?\n");
    code.push_str("    public let error: String?\n");
    code.push_str("}\n\n");
    
    code.push_str("public struct DiscoveredHost: Codable {\n");
    code.push_str("    public let hostname: String\n");
    code.push_str("    public let localIp: String?\n");
    code.push_str("    public let tailscaleIp: String?\n");
    code.push_str("    public let tailscaleHostname: String?\n");
    code.push_str("    public let agentPort: UInt16\n");
    code.push_str("    public let reachable: Bool\n");
    code.push_str("}\n\n");
    
    code.push_str("public struct HostInfo: Codable {\n");
    code.push_str("    public let dockerVersion: String?\n");
    code.push_str("    public let tailscaleInstalled: Bool\n");
    code.push_str("    public let portainerInstalled: Bool\n");
    code.push_str("}\n\n");
    
    code.push_str("public struct PingAgentRequest: Codable {\n");
    code.push_str("    public let host: String\n");
    code.push_str("    public let port: UInt16\n");
    code.push_str("}\n\n");
    
    code.push_str("public struct GetHostInfoRequest: Codable {\n");
    code.push_str("    public let host: String\n");
    code.push_str("    public let port: UInt16\n");
    code.push_str("}\n\n");
    
    code.push_str("public struct ExecuteCommandRequest: Codable {\n");
    code.push_str("    public let host: String\n");
    code.push_str("    public let port: UInt16\n");
    code.push_str("    public let command: String\n");
    code.push_str("    public let args: [String]\n");
    code.push_str("}\n\n");
    
    // Client class
    code.push_str("public class HalvorApiClient {\n");
    code.push_str("    private let baseUrl: String\n");
    code.push_str("    private let session: URLSession\n\n");
    code.push_str("    public init(baseUrl: String = \"http://localhost:8080\") {\n");
    code.push_str("        self.baseUrl = baseUrl\n");
    code.push_str("        self.session = URLSession.shared\n");
    code.push_str("    }\n\n");
    
    // Generate methods
    for endpoint in endpoints {
        let method_name = to_camel_case(&endpoint.handler);
        let return_type = map_rust_type_to_swift(&endpoint.response_type);
        
        code.push_str(&format!("    /// {}\n", endpoint.description));
        code.push_str(&format!("    /// - Returns: {}\n", return_type));
        code.push_str("    /// - Throws: Error if request fails\n");
        code.push_str("    public func ");
        
        if let Some(ref req_type) = endpoint.request_type {
            code.push_str(&format!("{}(request: {}) async throws -> {} {{\n", method_name, req_type, return_type));
            code.push_str(&format!("        let url = URL(string: \"$baseUrl{}\")!\n", endpoint.path));
            code.push_str("        var urlRequest = URLRequest(url: url)\n");
            code.push_str("        urlRequest.httpMethod = \"POST\"\n");
            code.push_str("        urlRequest.setValue(\"application/json\", forHTTPHeaderField: \"Content-Type\")\n");
            code.push_str("        urlRequest.httpBody = try JSONEncoder().encode(request)\n\n");
        } else {
            code.push_str(&format!("{}() async throws -> {} {{\n", method_name, return_type));
            code.push_str(&format!("        let url = URL(string: \"$baseUrl{}\")!\n", endpoint.path));
            code.push_str("        var urlRequest = URLRequest(url: url)\n");
            code.push_str("        urlRequest.httpMethod = \"GET\"\n");
        }
        
        code.push_str("        let (data, response) = try await session.data(for: urlRequest)\n\n");
        code.push_str("        guard let httpResponse = response as? HTTPURLResponse else {\n");
        code.push_str("            throw HalvorApiError.invalidResponse\n");
        code.push_str("        }\n\n");
        code.push_str("        guard httpResponse.statusCode == 200 else {\n");
        code.push_str("            throw HalvorApiError.httpError(httpResponse.statusCode)\n");
        code.push_str("        }\n\n");
        code.push_str(&format!("        let apiResponse: ApiResponse<{}> = try JSONDecoder().decode(ApiResponse<{}>.self, from: data)\n", return_type, return_type));
        code.push_str("        guard apiResponse.success, let result = apiResponse.data else {\n");
        code.push_str("            throw HalvorApiError.apiError(apiResponse.error ?? \"Unknown error\")\n");
        code.push_str("        }\n\n");
        code.push_str("        return result\n");
        code.push_str("    }\n\n");
    }
    
    code.push_str("}\n\n");
    code.push_str("public enum HalvorApiError: Error {\n");
    code.push_str("    case invalidResponse\n");
    code.push_str("    case httpError(Int)\n");
    code.push_str("    case apiError(String)\n");
    code.push_str("}\n");
    
    fs::write(output_dir.join("HalvorApiClient.swift"), code)
        .with_context(|| "Failed to write Swift client")?;
    
    Ok(())
}

/// Convert snake_case to camelCase
fn to_camel_case(snake: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Map Rust type to TypeScript type
fn map_rust_type_to_ts(rust_type: &str) -> String {
    match rust_type {
        "bool" => "boolean".to_string(),
        "String" => "string".to_string(),
        "u16" | "u32" | "u64" | "i32" | "i64" => "number".to_string(),
        _ if rust_type.starts_with("Vec<") => {
            let inner = rust_type.trim_start_matches("Vec<").trim_end_matches(">");
            format!("{}[]", map_rust_type_to_ts(inner))
        }
        _ => rust_type.to_string(),
    }
}

/// Map Rust type to Kotlin type
fn map_rust_type_to_kotlin(rust_type: &str) -> String {
    match rust_type {
        "bool" => "Boolean".to_string(),
        "String" => "String".to_string(),
        "u16" | "u32" | "u64" | "i32" | "i64" => "Int".to_string(),
        _ if rust_type.starts_with("Vec<") => {
            let inner = rust_type.trim_start_matches("Vec<").trim_end_matches(">");
            format!("List<{}>", map_rust_type_to_kotlin(inner))
        }
        _ => rust_type.to_string(),
    }
}

/// Map Rust type to Swift type
fn map_rust_type_to_swift(rust_type: &str) -> String {
    match rust_type {
        "bool" => "Bool".to_string(),
        "String" => "String".to_string(),
        "u16" => "UInt16".to_string(),
        "u32" => "UInt32".to_string(),
        "u64" => "UInt64".to_string(),
        "i32" => "Int32".to_string(),
        "i64" => "Int64".to_string(),
        _ if rust_type.starts_with("Vec<") => {
            let inner = rust_type.trim_start_matches("Vec<").trim_end_matches(">");
            format!("[{}]", map_rust_type_to_swift(inner))
        }
        _ => rust_type.to_string(),
    }
}

