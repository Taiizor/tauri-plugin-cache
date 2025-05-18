package app.tauri.plugin.cache;

import android.app.Activity;
import app.tauri.annotation.Command;
import app.tauri.annotation.TauriPlugin;
import app.tauri.plugin.JSObject;
import app.tauri.plugin.Plugin;
import app.tauri.plugin.PluginHandle;
import app.tauri.plugin.PluginMethod;

import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

@TauriPlugin
public class CachePlugin extends Plugin {
  private final Map<String, CacheEntry> cache = new ConcurrentHashMap<>();

  private static class CacheEntry {
    Object value;
    Long expiresAt;

    CacheEntry(Object value, Long expiresAt) {
      this.value = value;
      this.expiresAt = expiresAt;
    }
  }

  public void load() {
    // Start a cleanup thread
    startCleanupTask();
  }

  private void startCleanupTask() {
    Thread cleanupThread = new Thread(() -> {
      while (!Thread.currentThread().isInterrupted()) {
        try {
          Thread.sleep(60000); // Check every minute
          long now = System.currentTimeMillis() / 1000;
          
          // Clean up expired entries
          cache.entrySet().removeIf(entry -> {
            CacheEntry cacheEntry = entry.getValue();
            return cacheEntry.expiresAt != null && cacheEntry.expiresAt < now;
          });
        } catch (InterruptedException e) {
          Thread.currentThread().interrupt();
          break;
        }
      }
    });
    
    cleanupThread.setDaemon(true);
    cleanupThread.start();
  }

  @Command
  public Object set(PluginHandle handle, JSObject data) {
    try {
      String key = data.getString("key");
      Object value = data.opt("value");
      JSObject options = data.optJSObject("options");
      
      Long expiresAt = null;
      if (options != null && options.has("ttl")) {
        Long ttl = options.optLong("ttl", 0);
        if (ttl > 0) {
          expiresAt = (System.currentTimeMillis() / 1000) + ttl;
        }
      }
      
      cache.put(key, new CacheEntry(value, expiresAt));
      
      JSObject result = new JSObject();
      return result;
    } catch (Exception e) {
      JSObject error = new JSObject();
      error.put("message", e.getMessage());
      return error;
    }
  }

  @Command
  public Object get(PluginHandle handle, JSObject data) {
    try {
      String key = data.getString("key");
      CacheEntry entry = cache.get(key);
      
      if (entry == null) {
        return null;
      }
      
      // Check if expired
      if (entry.expiresAt != null) {
        long now = System.currentTimeMillis() / 1000;
        if (entry.expiresAt < now) {
          cache.remove(key);
          return null;
        }
      }
      
      return entry.value;
    } catch (Exception e) {
      JSObject error = new JSObject();
      error.put("message", e.getMessage());
      return error;
    }
  }

  @Command
  public JSObject has(PluginHandle handle, JSObject data) {
    try {
      String key = data.getString("key");
      CacheEntry entry = cache.get(key);
      
      boolean exists = false;
      if (entry != null) {
        // Check if expired
        if (entry.expiresAt != null) {
          long now = System.currentTimeMillis() / 1000;
          exists = entry.expiresAt >= now;
        } else {
          exists = true;
        }
      }
      
      JSObject result = new JSObject();
      result.put("value", exists);
      return result;
    } catch (Exception e) {
      JSObject error = new JSObject();
      error.put("message", e.getMessage());
      return error;
    }
  }

  @Command
  public JSObject remove(PluginHandle handle, JSObject data) {
    try {
      String key = data.getString("key");
      cache.remove(key);
      
      JSObject result = new JSObject();
      return result;
    } catch (Exception e) {
      JSObject error = new JSObject();
      error.put("message", e.getMessage());
      return error;
    }
  }

  @Command
  public JSObject clear(PluginHandle handle, JSObject data) {
    try {
      cache.clear();
      
      JSObject result = new JSObject();
      return result;
    } catch (Exception e) {
      JSObject error = new JSObject();
      error.put("message", e.getMessage());
      return error;
    }
  }

  @Command
  public JSObject stats(PluginHandle handle, JSObject data) {
    try {
      JSObject result = new JSObject();
      result.put("size", cache.size());
      return result;
    } catch (Exception e) {
      JSObject error = new JSObject();
      error.put("message", e.getMessage());
      return error;
    }
  }
}
