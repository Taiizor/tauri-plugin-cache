import SwiftRs
import Tauri
import UIKit
import Foundation
import Compression

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
        var methodUsed: UInt8 = 0
        
        if shouldCompress && valueData.count > compressionThreshold {
            // Choose compression method
            if compressionMethodToUse.lowercased() == "lzma2" {
                if let compressed = compressWithLZMA(data: valueData) {
                    finalData = compressed
                    isCompressed = true
                    methodUsed = 2
                } else {
                    // Fall back to Zlib
                    if let compressed = compressWithZlib(data: valueData) {
                        finalData = compressed
                        isCompressed = true
                        methodUsed = 1
                    } else {
                        finalData = valueData
                    }
                }
            } else {
                if let compressed = compressWithZlib(data: valueData) {
                    finalData = compressed
                    isCompressed = true
                    methodUsed = 1
                } else {
                    finalData = valueData
                }
            }
        } else {
            finalData = valueData
        }
        
        // Build data with compression header if compressed
        var storedData: Data
        if isCompressed {
            storedData = Data()
            storedData.append(1) // Compression indicator
            storedData.append(methodUsed) // Method marker
            storedData.append(finalData)
        } else {
            storedData = finalData
        }
        
        // Create cache entry
        var cacheEntry: [String: Any] = [
            "value": storedData.base64EncodedString(),
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
    
    // MARK: - Compression Methods using Apple's Compression framework
    
    /// Compress data using Zlib (via Apple's Compression framework)
    private func compressWithZlib(data: Data) -> Data? {
        return compressData(data, algorithm: COMPRESSION_ZLIB)
    }
    
    /// Compress data using LZMA (via Apple's Compression framework)
    /// Note: Apple's Compression framework uses LZMA, not LZMA2, but provides similar compression ratios
    private func compressWithLZMA(data: Data) -> Data? {
        return compressData(data, algorithm: COMPRESSION_LZMA)
    }
    
    /// Generic compression using Apple's Compression framework
    private func compressData(_ data: Data, algorithm: compression_algorithm) -> Data? {
        // Calculate destination buffer size (worst case: slightly larger than source)
        let destinationBufferSize = data.count + 1024
        let destinationBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: destinationBufferSize)
        defer { destinationBuffer.deallocate() }
        
        let sourceBuffer = [UInt8](data)
        
        let compressedSize = compression_encode_buffer(
            destinationBuffer,
            destinationBufferSize,
            sourceBuffer,
            data.count,
            nil,
            algorithm
        )
        
        guard compressedSize > 0 else {
            print("Compression failed for algorithm: \(algorithm)")
            return nil
        }
        
        let compressedData = Data(bytes: destinationBuffer, count: compressedSize)
        print("Compressed \(data.count) bytes to \(compressedSize) bytes using algorithm \(algorithm)")
        return compressedData
    }
    
    /// Decompress data based on the compression header
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
        let compressedData = data.subdata(in: 2..<data.count)
        
        let algorithm: compression_algorithm
        switch method {
        case 1:
            algorithm = COMPRESSION_ZLIB
        case 2:
            algorithm = COMPRESSION_LZMA
        default:
            throw PluginError(code: .invalidArgs, message: "Unknown compression method: \(method)")
        }
        
        guard let decompressedData = decompressData(compressedData, algorithm: algorithm) else {
            throw PluginError(code: .operationFailed, message: "Failed to decompress data")
        }
        
        return decompressedData
    }
    
    /// Generic decompression using Apple's Compression framework
    private func decompressData(_ data: Data, algorithm: compression_algorithm) -> Data? {
        // Estimate decompressed size (start with 4x, grow if needed)
        var destinationBufferSize = data.count * 4
        var destinationBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: destinationBufferSize)
        
        let sourceBuffer = [UInt8](data)
        
        var decompressedSize = compression_decode_buffer(
            destinationBuffer,
            destinationBufferSize,
            sourceBuffer,
            data.count,
            nil,
            algorithm
        )
        
        // If buffer was too small, try with larger buffer
        if decompressedSize == 0 || decompressedSize == destinationBufferSize {
            destinationBuffer.deallocate()
            destinationBufferSize = data.count * 16
            destinationBuffer = UnsafeMutablePointer<UInt8>.allocate(capacity: destinationBufferSize)
            
            decompressedSize = compression_decode_buffer(
                destinationBuffer,
                destinationBufferSize,
                sourceBuffer,
                data.count,
                nil,
                algorithm
            )
        }
        
        defer { destinationBuffer.deallocate() }
        
        guard decompressedSize > 0 else {
            print("Decompression failed for algorithm: \(algorithm)")
            return nil
        }
        
        let decompressedData = Data(bytes: destinationBuffer, count: decompressedSize)
        print("Decompressed \(data.count) bytes to \(decompressedSize) bytes using algorithm \(algorithm)")
        return decompressedData
    }
}

// MARK: - Plugin Init Function

@_cdecl("init_plugin_cache")
func initPlugin() -> Plugin {
    return CachePlugin()
}