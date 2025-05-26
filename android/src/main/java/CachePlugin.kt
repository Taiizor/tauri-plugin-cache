package app.tauri.plugin.cache

import android.app.Activity
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import org.json.JSONObject
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.zip.Deflater
import java.util.zip.Inflater
import org.tukaani.xz.XZInputStream
import org.tukaani.xz.XZOutputStream
import org.tukaani.xz.LZMA2Options

@InvokeArg
class ConfigureRequest {
    var default_compression: Boolean? = null
    var compression_level: Int? = null
    var compression_threshold: Int? = null
    var compression_method: String? = null
}

@InvokeArg
class SetRequest {
  lateinit var key: String
  var value: Any? = null
  var options: SetItemOptions? = null
}

@InvokeArg
class SetItemOptions {
  var ttl: Long? = null
  var compress: Boolean? = null
  var compressionMethod: String? = null
}

@InvokeArg
class GetRequest {
  lateinit var key: String
}

@InvokeArg
class HasRequest {
  lateinit var key: String
}

@InvokeArg
class RemoveRequest {
  lateinit var key: String
}

@InvokeArg
class CompressionConfig {
  var enabled: Boolean = true
  var level: Int = 6
  var threshold: Int = 1024
  var method: String = "zlib"
}

@TauriPlugin
class CachePlugin(private val activity: Activity): Plugin(activity) {
    private var cacheDir: File
    private var defaultCompression = true
    private var compressionLevel = 6
    private var compressionThreshold = 1024 // 1KB
    private var compressionMethod = "zlib"

    init {
        cacheDir = File(activity.cacheDir, "tauri_cache")
        if (!cacheDir.exists()) {
            cacheDir.mkdirs()
        }
        android.util.Log.i("CachePlugin", "Cache plugin initialized at ${cacheDir.absolutePath}")
    }

    @Command
    fun configure(invoke: Invoke) {
        try {
            val config = invoke.parseArgs(ConfigureRequest::class.java)
            
            if (config.default_compression != null) {
                defaultCompression = config.default_compression!!
            }
            if (config.compression_level != null) {
                compressionLevel = config.compression_level!!
            }
            if (config.compression_threshold != null) {
                compressionThreshold = config.compression_threshold!!
            }
            if (config.compression_method != null) {
                compressionMethod = config.compression_method!!
            }
            
            android.util.Log.i("CachePlugin", "Configure: compression=$defaultCompression, level=$compressionLevel, method=$compressionMethod")
            invoke.resolve()
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Configure error: ${e.message}")
            invoke.reject("Failed to configure cache: ${e.message}")
        }
    }

    @Command
    fun updateCompressionConfig(invoke: Invoke) {
        try {
            val config = invoke.parseArgs(CompressionConfig::class.java)
            defaultCompression = config.enabled
            compressionLevel = config.level
            compressionThreshold = config.threshold
            compressionMethod = config.method
            
            android.util.Log.i("CachePlugin", "Updated compression config: enabled=$defaultCompression, level=$compressionLevel, method=$compressionMethod")
            invoke.resolve()
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Update compression config error: ${e.message}")
            invoke.reject("Failed to update compression config: ${e.message}")
        }
    }

    @Command
    fun set(invoke: Invoke) {
        try {
            val request = invoke.parseArgs(SetRequest::class.java)
            android.util.Log.i("CachePlugin", "Setting cache item with key: ${request.key}")
            
            // Use the data - use request.value instead of invoke.data
            val valueString = if (request.value != null) {
                // Check if the value is JSON
                try {
                    // If it's a JSON object
                    if (request.value.toString().trim().startsWith("{") || 
                        request.value.toString().trim().startsWith("[")) {
                        // Save in JSON format
                        request.value.toString()
                    } else {
                        // Convert simple string values to JSON format
                        "\"" + request.value.toString().replace("\"", "\\\"") + "\""
                    }
                } catch (e: Exception) {
                    // In case of any error, treat as simple string
                    "\"" + request.value.toString().replace("\"", "\\\"") + "\""
                }
            } else {
                // Empty JSON object for null values
                "null"
            }
            
            android.util.Log.i("CachePlugin", "Value to save: $valueString")
            
            // Get TTL and compression information
            val ttl = request.options?.ttl
            val shouldCompress = request.options?.compress ?: defaultCompression
            val compressionMethodToUse = request.options?.compressionMethod ?: compressionMethod
            
            // Calculate expiration time
            val expiresAt = if (ttl != null) {
                System.currentTimeMillis() + (ttl * 1000)
            } else {
                null
            }
            
            // Create cache entry
            val entry = JSONObject()
            
            // Compress data (if needed)
            if (shouldCompress && valueString.length > compressionThreshold) {
                android.util.Log.i("CachePlugin", "Compressing data (${valueString.length} bytes) with $compressionMethodToUse")
                
                // Compress the data
                val compressed = compressData(valueString.toByteArray(), compressionMethodToUse)
                
                // Encode with Base64
                val encodedString = Base64.encodeToString(compressed, Base64.NO_WRAP)
                
                entry.put("value", encodedString)
                entry.put("is_compressed", true)
            } else {
                entry.put("value", valueString)
                entry.put("is_compressed", false)
            }
            
            // Add expiration time - explicitly specify Long type to resolve type ambiguity
            if (expiresAt != null) {
                entry.put("expires_at", expiresAt.toLong())
            }
            
            // Save to file
            val file = File(cacheDir, request.key)
            file.writeText(entry.toString())
            
            android.util.Log.i("CachePlugin", "Cache item saved to ${file.absolutePath}")
            
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Set error: ${e.message}")
            e.printStackTrace()
            invoke.reject("Failed to set item: ${e.message}")
        }
    }

    @Command
    fun get(invoke: Invoke) {
        try {
            val request = invoke.parseArgs(GetRequest::class.java)
            android.util.Log.i("CachePlugin", "Getting cache item with key: ${request.key}")
            
            val file = File(cacheDir, request.key)
            
            if (!file.exists()) {
                android.util.Log.i("CachePlugin", "Cache item not found: ${request.key}")
                invoke.resolve(null)
                return
            }
            
            // Read from file
            val entryJson = JSONObject(file.readText())
            
            // Check expiration date
            if (entryJson.has("expires_at")) {
                val expiresAt = entryJson.getLong("expires_at")
                
                if (System.currentTimeMillis() > expiresAt) {
                    android.util.Log.i("CachePlugin", "Cache item expired: ${request.key}")
                    file.delete()
                    invoke.resolve(null)
                    return
                }
            }
            
            // Check if compressed
            val isCompressed = entryJson.optBoolean("is_compressed", false)
            val value = entryJson.getString("value")
            
            if (isCompressed) {
                android.util.Log.i("CachePlugin", "Decompressing cache item: ${request.key}")
                
                // Decode from Base64
                val compressedData = Base64.decode(value, Base64.NO_WRAP)
                
                // Decompress the data
                val decompressedBytes = decompressData(compressedData)
                val decompressedString = String(decompressedBytes)
                
                // Convert content to JSObject - handle JSON parsing errors
                try {
                    // First check JSON format
                    val trimmedValue = decompressedString.trim()
                    if (trimmedValue.startsWith("{") && trimmedValue.endsWith("}") ||
                        trimmedValue.startsWith("[") && trimmedValue.endsWith("]")) {
                        // JSON object or array
                        val result = JSObject(decompressedString)
                        invoke.resolve(result)
                    } else if (trimmedValue.startsWith("\"") && trimmedValue.endsWith("\"")) {
                        // String value (in quotes)
                        val stringValue = trimmedValue.substring(1, trimmedValue.length - 1)
                        val result = JSObject()
                        result.put("value", stringValue)
                        invoke.resolve(result)
                    } else {
                        // Primitive value
                        val result = JSObject()
                        result.put("value", decompressedString)
                        invoke.resolve(result)
                    }
                } catch (e: Exception) {
                    // JSON parse error - return as simple string
                    android.util.Log.e("CachePlugin", "JSON parsing error, returning as string: ${e.message}")
                    val result = JSObject()
                    result.put("value", decompressedString)
                    invoke.resolve(result)
                }
            } else {
                // Uncompressed data - handle JSON parsing errors
                try {
                    // First check JSON format
                    val trimmedValue = value.trim()
                    if (trimmedValue.startsWith("{") && trimmedValue.endsWith("}") ||
                        trimmedValue.startsWith("[") && trimmedValue.endsWith("]")) {
                        // JSON object or array
                        val result = JSObject(value)
                        invoke.resolve(result)
                    } else if (trimmedValue.startsWith("\"") && trimmedValue.endsWith("\"")) {
                        // String value (in quotes)
                        val stringValue = trimmedValue.substring(1, trimmedValue.length - 1)
                        val result = JSObject()
                        result.put("value", stringValue)
                        invoke.resolve(result)
                    } else {
                        // Primitive value
                        val result = JSObject()
                        result.put("value", value)
                        invoke.resolve(result)
                    }
                } catch (e: Exception) {
                    // JSON parse error - return as simple string
                    android.util.Log.e("CachePlugin", "JSON parsing error, returning as string: ${e.message}")
                    val result = JSObject()
                    result.put("value", value)
                    invoke.resolve(result)
                }
            }
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Get error: ${e.message}")
            e.printStackTrace()
            invoke.reject("Failed to get item: ${e.message}")
        }
    }

    @Command
    fun has(invoke: Invoke) {
        try {
            val request = invoke.parseArgs(HasRequest::class.java)
            android.util.Log.i("CachePlugin", "Checking cache item with key: ${request.key}")
            
            val file = File(cacheDir, request.key)
            
            if (!file.exists()) {
                val result = JSObject()
                result.put("value", false)
                invoke.resolve(result)
                return
            }
            
            // Check expiration date
            try {
                val entryJson = JSONObject(file.readText())
                
                if (entryJson.has("expires_at")) {
                    val expiresAt = entryJson.getLong("expires_at")
                    
                    if (System.currentTimeMillis() > expiresAt) {
                        android.util.Log.i("CachePlugin", "Cache item expired: ${request.key}")
                        file.delete()
                        val result = JSObject()
                        result.put("value", false)
                        invoke.resolve(result)
                        return
                    }
                }
            } catch (e: Exception) {
                // Ignore if file cannot be read
                file.delete()
                val result = JSObject()
                result.put("value", false)
                invoke.resolve(result)
                return
            }
            
            val result = JSObject()
            result.put("value", true)
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Has error: ${e.message}")
            invoke.reject("Failed to check item: ${e.message}")
        }
    }

    @Command
    fun remove(invoke: Invoke) {
        try {
            val request = invoke.parseArgs(RemoveRequest::class.java)
            android.util.Log.i("CachePlugin", "Removing cache item with key: ${request.key}")
            
            val file = File(cacheDir, request.key)
            
            if (file.exists()) {
                file.delete()
                android.util.Log.i("CachePlugin", "Cache item removed: ${request.key}")
            }
            
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Remove error: ${e.message}")
            invoke.reject("Failed to remove item: ${e.message}")
        }
    }

    @Command
    fun clear(invoke: Invoke) {
        try {
            android.util.Log.i("CachePlugin", "Clearing all cache items")
            
            val files = cacheDir.listFiles()
            if (files != null) {
                for (file in files) {
                    file.delete()
                }
                android.util.Log.i("CachePlugin", "Removed ${files.size} cache items")
            }
            
            invoke.resolve(JSObject())
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Clear error: ${e.message}")
            invoke.reject("Failed to clear cache: ${e.message}")
        }
    }

    @Command
    fun stats(invoke: Invoke) {
        try {
            android.util.Log.i("CachePlugin", "Getting cache stats")
            
            val files = cacheDir.listFiles() ?: emptyArray()
            var totalSize = 0
            var activeSize = 0
            val now = System.currentTimeMillis()
            
            for (file in files) {
                totalSize++
                
                try {
                    val entryJson = JSONObject(file.readText())
                    
                    if (entryJson.has("expires_at")) {
                        val expiresAt = entryJson.getLong("expires_at")
                        
                        if (now <= expiresAt) {
                            activeSize++
                        }
                    } else {
                        activeSize++
                    }
                } catch (e: Exception) {
                    // Ignore if file cannot be read
                }
            }
            
            android.util.Log.i("CachePlugin", "Cache stats: total=$totalSize, active=$activeSize")
            
            val result = JSObject()
            result.put("totalSize", totalSize)
            result.put("activeSize", activeSize)
            invoke.resolve(result)
        } catch (e: Exception) {
            android.util.Log.e("CachePlugin", "Stats error: ${e.message}")
            invoke.reject("Failed to get stats: ${e.message}")
        }
    }
    
    // Data compression helper method
    private fun compressData(data: ByteArray, method: String): ByteArray {
        // Add compression markers: 1 = compressed, 1 = Zlib or 2 = LZMA2
        val outputStream = ByteArrayOutputStream(data.size)
        outputStream.write(1) // Compression marker
        
        // Check memory consumption - prevent using LZMA2 for very large data
        val maxLzma2Size = 10 * 1024 * 1024 // Don't use LZMA2 for data larger than 10MB
        
        // Safety check - if data is too large or despite LZMA2 being requested it exceeds memory limit
        // switch to Zlib which consumes less memory
        val useZlib = method.toLowerCase() != "lzma2" || data.size > maxLzma2Size
        
        if (!useZlib) {
            outputStream.write(2) // Method marker: 2 = LZMA2
            
            try {
                // Use settings that consume less memory for LZMA2
                val xzOutput = ByteArrayOutputStream(Math.min(data.size, 1024 * 1024)) // Max 1MB buffer
                
                // Configure LZMA2Options to reduce memory usage
                val xzEncoder = LZMA2Options(compressionLevel)
                xzEncoder.setDictSize(Math.min(1024 * 1024, data.size)) // Limit dictionary size (max 1MB)
                
                val xzOut = XZOutputStream(xzOutput, xzEncoder)
                
                // Process data in smaller chunks
                val chunkSize = 4096 // 4KB chunks
                var offset = 0
                while (offset < data.size) {
                    val length = Math.min(chunkSize, data.size - offset)
                    xzOut.write(data, offset, length)
                    offset += length
                    
                    // Clear buffer
                    if (offset % (512 * 1024) == 0) {
                        xzOut.flush()
                    }
                }
                
                xzOut.finish()
                xzOut.close()
                
                // Get compressed data
                val compressedData = xzOutput.toByteArray()
                outputStream.write(compressedData)
                
                android.util.Log.i("CachePlugin", "LZMA2 compressed ${data.size} bytes to ${compressedData.size} bytes")
            } catch (e: Exception) {
                android.util.Log.e("CachePlugin", "LZMA2 compression error: ${e.message}")
                // Fallback to Zlib on error
                outputStream.reset()
                outputStream.write(1) // Compression marker
                outputStream.write(1) // Method marker: 1 = Zlib
                
                compressWithZlib(data, outputStream)
            }
        } else {
            // Default Zlib compression
            outputStream.write(1) // Method marker: 1 = Zlib
            compressWithZlib(data, outputStream)
        }
        
        outputStream.close()
        return outputStream.toByteArray()
    }
    
    // Put Zlib compression process in a separate method
    private fun compressWithZlib(data: ByteArray, outputStream: ByteArrayOutputStream) {
        val deflater = Deflater(compressionLevel)
        deflater.setInput(data)
        deflater.finish()
        
        val buffer = ByteArray(4096) // Use larger buffer
        while (!deflater.finished()) {
            val count = deflater.deflate(buffer)
            outputStream.write(buffer, 0, count)
        }
        deflater.end()
    }
    
    // Data decompression helper method
    private fun decompressData(data: ByteArray): ByteArray {
        if (data.isEmpty()) {
            throw Exception("Empty data")
        }
        
        // First byte indicates whether it's compressed
        val isCompressed = data[0] == 1.toByte()
        
        if (!isCompressed) {
            // Return uncompressed data directly (skip marker bytes)
            return data.copyOfRange(2, data.size)
        }
        
        // Second byte indicates compression method
        val method = data[1].toInt()
        
        when (method) {
            1 -> {
                // Zlib decompression
                val inflater = Inflater()
                inflater.setInput(data, 2, data.size - 2)
                
                val outputStream = ByteArrayOutputStream(data.size * 2)
                val buffer = ByteArray(1024)
                
                while (!inflater.finished()) {
                    val count = inflater.inflate(buffer)
                    outputStream.write(buffer, 0, count)
                }
                
                outputStream.close()
                inflater.end()
                
                return outputStream.toByteArray()
            }
            2 -> {
                // LZMA2 decompression (using XZ Utils library)
                try {
                    val inputStream = ByteArrayInputStream(data, 2, data.size - 2)
                    val xzIn = XZInputStream(inputStream)
                    
                    val outputStream = ByteArrayOutputStream(data.size * 2)
                    val buffer = ByteArray(1024)
                    
                    var count = 0
                    while (xzIn.read(buffer).also { count = it } != -1) {
                        outputStream.write(buffer, 0, count)
                    }
                    
                    xzIn.close()
                    outputStream.close()
                    
                    return outputStream.toByteArray()
                } catch (e: Exception) {
                    android.util.Log.e("CachePlugin", "LZMA2 decompression error: ${e.message}")
                    throw Exception("LZMA2 decompression failed: ${e.message}")
                }
            }
            else -> {
                throw Exception("Unknown compression method: $method")
            }
        }
    }
}