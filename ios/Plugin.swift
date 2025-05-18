import Foundation
import SwiftRs
import Tauri

@objc(CachePlugin)
public class CachePlugin: Plugin {
    private struct CacheEntry {
        let value: Any
        let expiresAt: Int64?
    }
    
    private var cache: [String: CacheEntry] = [:]
    private let cleanupInterval: Int64 = 60 // seconds
    
    override init() {
        super.init()
        startCleanupTask()
    }
    
    private func startCleanupTask() {
        DispatchQueue.global(qos: .background).async { [weak self] in
            guard let self = self else { return }
            
            while true {
                Thread.sleep(forTimeInterval: TimeInterval(self.cleanupInterval))
                
                let now = Int64(Date().timeIntervalSince1970)
                
                // Lock for thread safety
                objc_sync_enter(self.cache)
                
                // Remove expired entries
                for (key, entry) in self.cache {
                    if let expiresAt = entry.expiresAt, expiresAt < now {
                        self.cache.removeValue(forKey: key)
                    }
                }
                
                objc_sync_exit(self.cache)
            }
        }
    }

    @objc func set(_ invoke: Invoke) throws -> NSNumber {
        let args = invoke.args
        
        guard let key = args["key"] as? String else {
            throw PluginError.invalidArguments("key must be a string")
        }
        
        let value = args["value"]
        
        var expiresAt: Int64? = nil
        if let options = args["options"] as? [String: Any],
           let ttl = options["ttl"] as? Int64,
           ttl > 0 {
            expiresAt = Int64(Date().timeIntervalSince1970) + ttl
        }
        
        objc_sync_enter(cache)
        cache[key] = CacheEntry(value: value as Any, expiresAt: expiresAt)
        objc_sync_exit(cache)
        
        invoke.resolve([:])
        return NSNumber(value: 0) // Success
    }
    
    @objc func get(_ invoke: Invoke) throws -> NSNumber {
        let args = invoke.args
        
        guard let key = args["key"] as? String else {
            throw PluginError.invalidArguments("key must be a string")
        }
        
        objc_sync_enter(cache)
        let entry = cache[key]
        objc_sync_exit(cache)
        
        if let entry = entry {
            if let expiresAt = entry.expiresAt {
                let now = Int64(Date().timeIntervalSince1970)
                if expiresAt < now {
                    // Expired
                    objc_sync_enter(cache)
                    cache.removeValue(forKey: key)
                    objc_sync_exit(cache)
                    
                    invoke.resolve(nil)
                    return NSNumber(value: 0)
                }
            }
            
            invoke.resolve(entry.value)
        } else {
            invoke.resolve(nil)
        }
        
        return NSNumber(value: 0)
    }
    
    @objc func has(_ invoke: Invoke) throws -> NSNumber {
        let args = invoke.args
        
        guard let key = args["key"] as? String else {
            throw PluginError.invalidArguments("key must be a string")
        }
        
        objc_sync_enter(cache)
        let entry = cache[key]
        objc_sync_exit(cache)
        
        var exists = false
        
        if let entry = entry {
            if let expiresAt = entry.expiresAt {
                let now = Int64(Date().timeIntervalSince1970)
                exists = expiresAt >= now
            } else {
                exists = true
            }
        }
        
        invoke.resolve(["value": exists])
        return NSNumber(value: 0)
    }
    
    @objc func remove(_ invoke: Invoke) throws -> NSNumber {
        let args = invoke.args
        
        guard let key = args["key"] as? String else {
            throw PluginError.invalidArguments("key must be a string")
        }
        
        objc_sync_enter(cache)
        cache.removeValue(forKey: key)
        objc_sync_exit(cache)
        
        invoke.resolve([:])
        return NSNumber(value: 0)
    }
    
    @objc func clear(_ invoke: Invoke) throws -> NSNumber {
        objc_sync_enter(cache)
        cache.removeAll()
        objc_sync_exit(cache)
        
        invoke.resolve([:])
        return NSNumber(value: 0)
    }
    
    @objc func stats(_ invoke: Invoke) throws -> NSNumber {
        objc_sync_enter(cache)
        let size = cache.count
        objc_sync_exit(cache)
        
        invoke.resolve(["size": size])
        return NSNumber(value: 0)
    }
}
