import SwiftRs
import Tauri
import UIKit
import Foundation
import Compression
import LZMA // Import LZMA library

// MARK: - Structures and Models

class ConfigureRequest: Decodable {
    let default_compression: Bool?
    let compression_level: Int?
    let compression_threshold: Int?
    let compression_method: String?
}

class CompressionConfig: Decodable {
    let enabled: Bool
    let level: Int
    let threshold: Int
    let method: String
}

class SetRequest<T: Decodable>: Decodable {
    let key: String
    let value: T
    let options: SetItemOptions?
}

class SetItemOptions: Decodable {
    let ttl: TimeInterval?
    let compress: Bool?
    let compressionMethod: String?
}

class GetRequest: Decodable {
    let key: String
}

class HasRequest: Decodable {
    let key: String
}

class RemoveRequest: Decodable {
    let key: String
}

class EmptyResponse: Encodable {
}

class BooleanResponse: Encodable {
    let value: Bool
    
    init(value: Bool) {
        self.value = value
    }
}

class CacheStats: Encodable {
    let totalSize: Int
    let activeSize: Int
    
    init(totalSize: Int, activeSize: Int) {
        self.totalSize = totalSize
        self.activeSize = activeSize
    }
}

// MARK: - Cache Plugin

class CachePlugin: Plugin {
    private let cacheDirectory: URL
    private var defaultCompression = true
    private var compressionLevel = 6
    private var compressionThreshold = 1024 // 1KB
    private var compressionMethod = "zlib"
    
    private let fileManager = FileManager.default
    private let syncQueue = DispatchQueue(label: "app.tauri.plugin.cache.sync")
    
    override init() {
        // Create cache directory
        let appCacheDir = fileManager.urls(for: .cachesDirectory, in: .userDomainMask).first!
        cacheDirectory = appCacheDir.appendingPathComponent("tauri_cache", isDirectory: true)
        
        super.init()
        
        // Create directory if it doesn't exist
        if !fileManager.fileExists(atPath: cacheDirectory.path) {
            try? fileManager.createDirectory(at: cacheDirectory, withIntermediateDirectories: true)
        }
        
        print("Cache plugin initialized at \(cacheDirectory.path)")
    }
    
    // MARK: - API Methods
    
    @objc public func configure(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(ConfigureRequest.self)
        
        if let defaultCompression = args.default_compression {
            self.defaultCompression = defaultCompression
        }
        
        if let compressionLevel = args.compression_level {
            self.compressionLevel = compressionLevel
        }
        
        if let compressionThreshold = args.compression_threshold {
            self.compressionThreshold = compressionThreshold
        }
        
        if let compressionMethod = args.compression_method {
            self.compressionMethod = compressionMethod
        }
        
        print("Configure: compression=\(defaultCompression), level=\(compressionLevel), method=\(compressionMethod)")
        invoke.resolve()
    }
    
    @objc public func updateCompressionConfig(_ invoke: Invoke) throws {
        let config = try invoke.parseArgs(CompressionConfig.self)
        self.defaultCompression = config.enabled
        self.compressionLevel = config.level
        self.compressionThreshold = config.threshold
        self.compressionMethod = config.method
        
        print("Updated compression config: enabled=\(defaultCompression), level=\(compressionLevel), method=\(compressionMethod)")
        invoke.resolve()
    }
    
    @objc public func set(_ invoke: Invoke) throws {
        // A slightly different approach to decode generic types in Swift
        let json = invoke.argsJson
        guard let jsonData = json.data(using: .utf8) else {
            throw PluginError(code: .invalidArgs, message: "Invalid JSON data")
        }
        
        guard let dict = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any],
              let key = dict["key"] as? String,
              dict["value"] != nil else {
            throw PluginError(code: .invalidArgs, message: "Missing key or value")
        }
        
        // Extract options separately
        var ttl: TimeInterval? = nil
        var shouldCompress = self.defaultCompression
        var compressionMethodToUse = self.compressionMethod
        
        if let options = dict["options"] as? [String: Any] {
            ttl = options["ttl"] as? TimeInterval
            if let compress = options["compress"] as? Bool {
                shouldCompress = compress
            }
            if let method = options["compressionMethod"] as? String {
                compressionMethodToUse = method
            }
        }
        
        // Store as JSON data
        let valueData: Data
        if let value = dict["value"] {
            valueData = try JSONSerialization.data(withJSONObject: value)
        } else {
            valueData = "null".data(using: .utf8)!
        }
        
        // Apply compression
        let finalData: Data
        var isCompressed = false
        
        if shouldCompress && valueData.count > compressionThreshold {
            // Choose compression method
            if compressionMethodToUse.lowercased() == "lzma2" {
                finalData = compressWithLZMA2(data: valueData)
                isCompressed = true
            } else {
                finalData = compressWithZlib(data: valueData)
                isCompressed = true
            }
        } else {
            finalData = valueData
        }
        
        // Create cache entry
        var cacheEntry: [String: Any] = [
            "value": finalData.base64EncodedString(),
            "is_compressed": isCompressed
        ]
        
        // Add expiration time
        if let ttl = ttl {
            let expiresAt = Date().timeIntervalSince1970 + ttl
            cacheEntry["expires_at"] = expiresAt
        }
        
        // Save as JSON
        let entryData = try JSONSerialization.data(withJSONObject: cacheEntry)
        
        // Save the file
        let fileURL = cacheDirectory.appendingPathComponent(key)
        syncQueue.sync {
            do {
                try entryData.write(to: fileURL)
                print("Cache item saved to \(fileURL.path)")
            } catch {
                print("Failed to write cache file: \(error)")
            }
        }
        
        invoke.resolve(EmptyResponse())
    }
    
    @objc public func get(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(GetRequest.self)
        let key = args.key
        
        let fileURL = cacheDirectory.appendingPathComponent(key)
        
        // Check if file exists
        guard fileManager.fileExists(atPath: fileURL.path) else {
            invoke.resolveNil()
            return
        }
        
        // Read the file
        let data: Data
        do {
            data = try Data(contentsOf: fileURL)
        } catch {
            print("Failed to read cache file: \(error)")
            invoke.resolveNil()
            return
        }
        
        // Parse as JSON
        guard let entryDict = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            invoke.resolveNil()
            return
        }
        
        // Check expiration time
        if let expiresAt = entryDict["expires_at"] as? TimeInterval {
            let now = Date().timeIntervalSince1970
            if now > expiresAt {
                // Item expired, delete it
                try? fileManager.removeItem(at: fileURL)
                invoke.resolveNil()
                return
            }
        }
        
        // Extract value
        guard let valueBase64 = entryDict["value"] as? String,
              let valueData = Data(base64Encoded: valueBase64) else {
            invoke.resolveNil()
            return
        }
        
        // Check if compressed
        let isCompressed = entryDict["is_compressed"] as? Bool ?? false
        
        let finalData: Data
        if isCompressed {
            do {
                finalData = try decompressData(valueData)
            } catch {
                print("Failed to decompress data: \(error)")
                invoke.resolveNil()
                return
            }
        } else {
            finalData = valueData
        }
        
        // Parse JSON data
        if let jsonObject = try? JSONSerialization.jsonObject(with: finalData),
           let jsonString = String(data: try JSONSerialization.data(withJSONObject: jsonObject), encoding: .utf8) {
            invoke.resolveString(jsonString)
        } else if let stringValue = String(data: finalData, encoding: .utf8) {
            // Accept as direct string
            invoke.resolveString(stringValue)
        } else {
            invoke.resolveNil()
        }
    }
    
    @objc public func has(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(HasRequest.self)
        let key = args.key
        
        let fileURL = cacheDirectory.appendingPathComponent(key)
        
        // Check if file exists
        guard fileManager.fileExists(atPath: fileURL.path) else {
            invoke.resolve(BooleanResponse(value: false))
            return
        }
        
        // Read file and check validity
        do {
            let data = try Data(contentsOf: fileURL)
            guard let entryDict = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                invoke.resolve(BooleanResponse(value: false))
                return
            }
            
            // Check expiration time
            if let expiresAt = entryDict["expires_at"] as? TimeInterval {
                let now = Date().timeIntervalSince1970
                if now > expiresAt {
                    // Item expired, delete it
                    try? fileManager.removeItem(at: fileURL)
                    invoke.resolve(BooleanResponse(value: false))
                    return
                }
            }
            
            invoke.resolve(BooleanResponse(value: true))
        } catch {
            // Could not read file
            invoke.resolve(BooleanResponse(value: false))
        }
    }
    
    @objc public func remove(_ invoke: Invoke) throws {
        let args = try invoke.parseArgs(RemoveRequest.self)
        let key = args.key
        
        let fileURL = cacheDirectory.appendingPathComponent(key)
        
        if fileManager.fileExists(atPath: fileURL.path) {
            do {
                try fileManager.removeItem(at: fileURL)
                print("Cache item removed: \(key)")
            } catch {
                print("Failed to remove cache item: \(error)")
            }
        }
        
        invoke.resolve(EmptyResponse())
    }
    
    @objc public func clear(_ invoke: Invoke) throws {
        do {
            let contents = try fileManager.contentsOfDirectory(at: cacheDirectory, includingPropertiesForKeys: nil)
            for fileURL in contents {
                try fileManager.removeItem(at: fileURL)
            }
            print("Removed \(contents.count) cache items")
        } catch {
            print("Failed to clear cache: \(error)")
        }
        
        invoke.resolve(EmptyResponse())
    }
    
    @objc public func stats(_ invoke: Invoke) throws {
        var totalSize = 0
        var activeSize = 0
        let now = Date().timeIntervalSince1970
        
        do {
            let contents = try fileManager.contentsOfDirectory(at: cacheDirectory, includingPropertiesForKeys: nil)
            totalSize = contents.count
            
            for fileURL in contents {
                do {
                    let data = try Data(contentsOf: fileURL)
                    if let entryDict = try JSONSerialization.jsonObject(with: data) as? [String: Any] {
                        if let expiresAt = entryDict["expires_at"] as? TimeInterval {
                            if now <= expiresAt {
                                activeSize += 1
                            }
                        } else {
                            activeSize += 1
                        }
                    }
                } catch {
                    // Could not read file, skip
                }
            }
        } catch {
            print("Failed to get stats: \(error)")
        }
        
        invoke.resolve(CacheStats(totalSize: totalSize, activeSize: activeSize))
    }
    
    // MARK: - Helper Methods
    
    // Compression with Zlib
    private func compressWithZlib(data: Data) -> Data {
        var compressedData = Data()
        
        // Compression indicator: 1 = compressed, 1 = Zlib
        compressedData.append(1)
        compressedData.append(1)
        
        // Compress data with Zlib in Swift
        do {
            // Data compression in iOS
            let zlibData = try data.deflated(level: compressionLevel)
            compressedData.append(zlibData)
            
            print("Zlib compressed \(data.count) bytes to \(zlibData.count) bytes")
            return compressedData
        } catch {
            print("Zlib compression error: \(error)")
            // Return uncompressed data in case of simple error
            return data
        }
    }
    
    // Compression with LZMA2 (using LZMA library)
    private func compressWithLZMA2(data: Data) -> Data {
        var compressedData = Data()
        
        // Memory limitation for very large data
        let maxLzma2Size = 10 * 1024 * 1024 // Don't use for data larger than 10MB
        
        if data.count > maxLzma2Size {
            print("Data too large for LZMA2 compression, using Zlib instead")
            return compressWithZlib(data: data)
        }
        
        // Compression indicator: 1 = compressed, 2 = LZMA2
        compressedData.append(1)
        compressedData.append(2)
        
        do {
            // Compression using LZMA library
            let lzmaConfig = LZMAConfig()
            lzmaConfig.level = UInt32(min(9, max(0, compressionLevel)))
            lzmaConfig.dictionarySize = UInt32(min(1024 * 1024, data.count)) // Limit dictionary size to 1MB
            
            let lzmaData = try LZMA.compress(data: data, config: lzmaConfig)
            compressedData.append(lzmaData)
            
            print("LZMA2 compressed \(data.count) bytes to \(lzmaData.count) bytes")
            return compressedData
        } catch {
            print("LZMA2 compression error: \(error)")
            // Fall back to Zlib in case of error
            return compressWithZlib(data: data)
        }
    }
    
    // Decompress data
    private func decompressData(_ data: Data) throws -> Data {
        guard data.count >= 2 else {
            throw PluginError(code: .invalidArgs, message: "Invalid compressed data")
        }
        
        // First byte is compression indicator
        let isCompressed = data[0] == 1
        
        if !isCompressed {
            // Uncompressed data
            return data.subdata(in: 2..<data.count)
        }
        
        // Second byte is compression method
        let method = data[1]
        
        switch method {
        case 1:
            // Zlib decompression
            let zlibData = data.subdata(in: 2..<data.count)
            do {
                let decompressedData = try zlibData.inflated()
                return decompressedData
            } catch {
                throw PluginError(code: .operationFailed, message: "Failed to decompress Zlib data: \(error)")
            }
            
        case 2:
            // LZMA2 decompression
            let lzmaData = data.subdata(in: 2..<data.count)
            do {
                let decompressedData = try LZMA.decompress(data: lzmaData)
                return decompressedData
            } catch {
                throw PluginError(code: .operationFailed, message: "Failed to decompress LZMA2 data: \(error)")
            }
            
        default:
            throw PluginError(code: .invalidArgs, message: "Unknown compression method: \(method)")
        }
    }
}

// MARK: - Data Extensions

extension Data {
    // Zlib compression extension
    func deflated(level: Int = 6) throws -> Data {
        let destSize = self.count / 4 + 1024 // Sufficiently large buffer
        var dest = [UInt8](repeating: 0, count: destSize)
        var destLen = UInt(destSize)
        
        let source = [UInt8](self)
        
        let result = compression_encode_buffer(&dest, destLen, source, UInt(source.count),
                                            nil, COMPRESSION_ZLIB)
        
        if result == 0 {
            throw PluginError(code: .operationFailed, message: "Zlib compression failed")
        }
        
        return Data(dest[0..<Int(destLen)])
    }
    
    // Zlib decompression extension
    func inflated() throws -> Data {
        let destSize = self.count * 4 // Decompressed data is usually larger
        var dest = [UInt8](repeating: 0, count: destSize)
        var destLen = UInt(destSize)
        
        let source = [UInt8](self)
        
        let result = compression_decode_buffer(&dest, destLen, source, UInt(source.count),
                                            nil, COMPRESSION_ZLIB)
        
        if result == 0 {
            throw PluginError(code: .operationFailed, message: "Zlib decompression failed")
        }
        
        return Data(dest[0..<Int(destLen)])
    }
}

// MARK: - Plugin Init Function

@_cdecl("init_plugin_cache")
func initPlugin() -> Plugin {
    return CachePlugin()
} 