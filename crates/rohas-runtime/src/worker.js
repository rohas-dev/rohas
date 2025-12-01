#!/usr/bin/env node

import { createRequire } from 'module';
import readline from 'readline';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import { dirname } from 'path';
import vm from 'vm';

const require = createRequire(import.meta.url);

// RPC Protocol: JSON-RPC over stdin/stdout
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
  terminal: false
});

const moduleCache = new Map();

class Logger {
  constructor(handlerName, logFn) {
    this.handlerName = handlerName;
    this.logFn = logFn;
  }
  info(message, fields) {
    try {
      if (this.logFn) {
        this.logFn("info", this.handlerName, message, fields || {});
      } else {
        console.error('[Logger] logFn is not defined for info');
      }
    } catch (error) {
      console.error('[Logger] Error in info:', error);
    }
  }
  error(message, fields) {
    try {
      if (this.logFn) {
        this.logFn("error", this.handlerName, message, fields || {});
      } else {
        console.error('[Logger] logFn is not defined for error');
      }
    } catch (error) {
      console.error('[Logger] Error in error:', error);
    }
  }
  warning(message, fields) {
    try {
      if (this.logFn) {
        this.logFn("warn", this.handlerName, message, fields || {});
      } else {
        console.error('[Logger] logFn is not defined for warning');
      }
    } catch (error) {
      console.error('[Logger] Error in warning:', error);
    }
  }
  warn(message, fields) {
    this.warning(message, fields);
  }
  debug(message, fields) {
    try {
      if (this.logFn) {
        this.logFn("debug", this.handlerName, message, fields || {});
      } else {
        console.error('[Logger] logFn is not defined for debug');
      }
    } catch (error) {
      console.error('[Logger] Error in debug:', error);
    }
  }
  trace(message, fields) {
    try {
      if (this.logFn) {
        this.logFn("trace", this.handlerName, message, fields || {});
      } else {
        console.error('[Logger] logFn is not defined for trace');
      }
    } catch (error) {
      console.error('[Logger] Error in trace:', error);
    }
  }
}

class State {
  constructor(handlerName, logFn) {
    this.triggers = [];
    this.autoTriggerPayloads = new Map();
    this.logger = new Logger(handlerName || "unknown", logFn);
  }
  triggerEvent(eventName, payload) {
    this.triggers.push({ eventName, payload });
  }
  setPayload(eventName, payload) {
    this.autoTriggerPayloads.set(eventName, payload);
  }
  getTriggers() {
    return [...this.triggers];
  }
  getAutoTriggerPayload(eventName) {
    return this.autoTriggerPayloads.get(eventName);
  }
  getAllAutoTriggerPayloads() {
    return Object.fromEntries(this.autoTriggerPayloads);
  }
}

// Logs collection
let logs = [];

function logFn(level, handler, message, fields) {
  logs.push({
    level,
    handler,
    message,
    fields: fields || {},
    timestamp: new Date().toISOString()
  });
}

async function instantiateRequestObject(handlerName, context, projectRequire) {
  try {
    const toPascalCase = (str) => {
      return str.split('_').map(word => 
        word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
      ).join('');
    };
    
    const requestClassName = `${toPascalCase(handlerName)}Request`;
    const moduleName = handlerName.toLowerCase();
    
    try {
      const apiModule = await import(`@generated/api/${moduleName}`);
      if (apiModule[requestClassName]) {
        const requestData = {
          ...context.payload,
          queryParams: context.query_params || {}
        };
        
        if (requestData.body) {
          return new apiModule[requestClassName](requestData);
        } else {
          return new apiModule[requestClassName](requestData);
        }
      }
    } catch (importError) {
      console.error(`[DEBUG] Failed to import request class ${requestClassName}:`, importError.message);
    }
  } catch (error) {
    console.error(`[DEBUG] Error instantiating request object:`, error.message);
  }
  
  // Fallback: return plain object with payload and queryParams
  return {
    ...context.payload,
    queryParams: context.query_params || {}
  };
}

// Helper function to instantiate event object from generated event module
async function instantiateEventObject(eventName, context, projectRequire) {
  try {
    const toSnakeCase = (str) => {
      return str.replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '');
    };
    
    const eventNameSnake = toSnakeCase(eventName);
    const payloadType = context.metadata?.event_payload_type;
    
    // Try to import from generated event module
    try {
      const eventModule = await import(`@generated/events/${eventNameSnake}`);
      if (eventModule[eventName]) {
        // Build event payload
        let payload = context.payload;
        
        // If payload type is specified and not primitive, try to instantiate payload model
        if (payloadType && !isPrimitiveType(payloadType)) {
          try {
            const payloadTypeSnake = toSnakeCase(payloadType);
            const modelModule = await import(`@generated/models/${payloadTypeSnake}`);
            if (modelModule[payloadType]) {
              payload = new modelModule[payloadType](payload);
            }
          } catch (modelError) {
            // Fallback to plain payload
            console.error(`[DEBUG] Failed to instantiate payload model ${payloadType}:`, modelError.message);
          }
        }
        
        // Create event object with payload and timestamp
        return new eventModule[eventName]({
          payload,
          timestamp: new Date()
        });
      }
    } catch (importError) {
      console.error(`[DEBUG] Failed to import event class ${eventName}:`, importError.message);
    }
  } catch (error) {
    console.error(`[DEBUG] Error instantiating event object:`, error.message);
  }
  
  // Fallback: return plain object
  return {
    payload: context.payload,
    timestamp: new Date()
  };
}

async function callWebSocketHandler(handlerFn, context, paramCount, state, projectRequire) {
  const wsName = context.metadata?.websocket_name || 'HelloWorld';
  const toSnakeCase = (str) => {
    return str.replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '');
  };
  
  const wsNameSnake = toSnakeCase(wsName);
  const connectionClassName = `${wsName}Connection`;
  const messageClassName = `${wsName}Message`;
  
  let connectionObj = context.payload?.connection || context.payload;
  let messageObj = null;
  
  try {
    const wsModule = await import(`@generated/websockets/${wsNameSnake}`);
    if (wsModule[connectionClassName]) {
      const connectionData = context.payload?.connection || context.payload;
      connectionObj = new wsModule[connectionClassName](connectionData);
    }
    
    // Try to instantiate message object if handler has 3 parameters
    if (paramCount >= 3 && wsModule[messageClassName]) {
      const messageData = context.payload?.message || context.payload;
      messageObj = new wsModule[messageClassName](messageData);
    }
  } catch (importError) {
    // Fallback to plain objects
    console.error(`[DEBUG] Failed to import WebSocket classes:`, importError.message);
  }
  
  if (paramCount === 3) {
    if (messageObj) {
      return await handlerFn(messageObj, connectionObj, state);
    } else {
      return await handlerFn(context.payload?.message || context.payload, connectionObj, state);
    }
  } else if (paramCount === 2) {
    return await handlerFn(connectionObj, state);
  } else {
    return await handlerFn(connectionObj);
  }
}

function isPrimitiveType(typeName) {
  return ['String', 'Int', 'Float', 'Boolean', 'Bool', 'DateTime', 'Date'].includes(typeName);
}

async function executeHandler(handlerPath, context) {
  logs = []; // Reset logs for each invocation
  
  try {
    let resolvedPath = handlerPath;
    if (!path.isAbsolute(handlerPath)) {
      resolvedPath = path.resolve(process.cwd(), handlerPath);
    }

    if (resolvedPath.endsWith('.ts') || resolvedPath.endsWith('.tsx')) {
      let compiledPath = null;
      
      if (resolvedPath.includes('/src/')) {
        compiledPath = resolvedPath.replace('/src/', '/.rohas/').replace(/\.tsx?$/, '.js');
      } else if (resolvedPath.includes('src/')) {
        compiledPath = resolvedPath.replace('src/', '.rohas/').replace(/\.tsx?$/, '.js');
      } else {
        // Try relative to cwd
        const relativePath = path.relative(process.cwd(), resolvedPath);
        if (relativePath.startsWith('src/')) {
          compiledPath = path.join(process.cwd(), relativePath.replace('src/', '.rohas/').replace(/\.tsx?$/, '.js'));
        }
      }
      
      if (compiledPath && fs.existsSync(compiledPath)) {
        resolvedPath = compiledPath;
      } else {
        const suggestedPath = compiledPath || resolvedPath.replace(/\.tsx?$/, '.js');
        throw new Error(
          `TypeScript handler not compiled.\n` +
          `  Source: ${handlerPath}\n` +
          `  Expected compiled: ${suggestedPath}\n` +
          `  Please run 'npm run compile' or 'rspack build' to compile TypeScript files.`
        );
      }
    } else if (!resolvedPath.endsWith('.js')) {
      resolvedPath += '.js';
    }
    
    if (!fs.existsSync(resolvedPath)) {
      throw new Error(`Handler file not found: ${resolvedPath}. Original path: ${handlerPath}`);
    }
    
    let handlerModule;
    
    const absolutePath = path.isAbsolute(resolvedPath) 
      ? resolvedPath 
      : path.resolve(process.cwd(), resolvedPath);
    
    const projectRoot = process.cwd();
    const projectRequire = createRequire(path.join(projectRoot, 'package.json'));
     
    try {
      const code = fs.readFileSync(absolutePath, 'utf8');
      const mod = { exports: {} };
      
      const vmRequire = (moduleId) => {
        try {
          return projectRequire(moduleId);
        } catch (requireError) {
          throw new Error(`Cannot find module '${moduleId}'. Make sure it's installed in node_modules. ${requireError.message}`);
        }
      };
      
      vmRequire.resolve = (moduleId) => {
        try {
          return projectRequire.resolve(moduleId);
        } catch (resolveError) {
          throw new Error(`Cannot resolve module '${moduleId}'. Make sure it's installed in node_modules. ${resolveError.message}`);
        }
      };
      
      vmRequire.cache = projectRequire.cache;
      vmRequire.extensions = projectRequire.extensions;
      
      const context = vm.createContext({
        module: mod,
        exports: mod.exports,
        require: vmRequire,
        __dirname: path.dirname(absolutePath),
        __filename: absolutePath,
        process: process,
        global: globalThis,
        console: console,
        Buffer: Buffer,
        setTimeout: setTimeout,
        clearTimeout: clearTimeout,
        setInterval: setInterval,
        clearInterval: clearInterval
      });
      
      vm.runInContext(code, context);
      handlerModule = mod.exports;
      
      const exportKeys = Object.keys(handlerModule);
      const allKeys = Object.getOwnPropertyNames(handlerModule);
      
      if (!handlerModule || (exportKeys.length === 0 && allKeys.length === 0)) {
        throw new Error(`Module loaded but has no exports. This might be a webpack/rspack bundling issue.`);
      }
      
      if (process.env.NODE_ENV !== 'production') {
        console.error(`[DEBUG] Loaded module exports:`, exportKeys.length > 0 ? exportKeys : allKeys);
      }
    } catch (vmError) {
      try {
        delete require.cache[require.resolve(absolutePath)];
        handlerModule = require(absolutePath);
      } catch (requireError) {
        if (requireError.code === 'ERR_REQUIRE_ESM' || requireError.message.includes('ES Module')) {
          try {
            const fileUrl = process.platform === 'win32'
              ? `file:///${absolutePath.replace(/\\/g, '/')}`
              : `file://${absolutePath}`;
            const imported = await import(fileUrl);
            handlerModule = imported.default || imported;
          } catch (importError) {
            throw new Error(`Failed to load handler module. VM: ${vmError.message}, Require: ${requireError.message}, Import: ${importError.message}. Path: ${resolvedPath}`);
          }
        } else {
          throw new Error(`Failed to load handler module. VM: ${vmError.message}, Require: ${requireError.message}. Path: ${resolvedPath}`);
        }
      }
    }
    
    const pathParts = absolutePath.split(path.sep);
    const handlersIndex = pathParts.indexOf('handlers');
    const handlerType = handlersIndex >= 0 && handlersIndex < pathParts.length - 1
      ? pathParts[handlersIndex + 1]
      : null;
    
    const isEventHandler = handlerType === 'events';
    const isWebSocketHandler = handlerType === 'websockets';
    const isMiddleware = handlerType === 'middlewares';
    const isCronHandler = handlerType === 'cron';

    let handlerFn;
    const handlerName = context.handler_name || 'handler';

    const toCamelCase = (str) => {
      if (!str) return str;
      return str.charAt(0).toLowerCase() + str.slice(1);
    };
    
    const toSnakeCase = (str) => {
      return str.replace(/([A-Z])/g, '_$1').toLowerCase().replace(/^_/, '');
    };
    
    const toPascalCase = (str) => {
      return str.split('_').map(word => 
        word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()
      ).join('');
    };
    

    let functionName;
    if (isEventHandler || isWebSocketHandler) {
      const directName = handlerName;
      const handleName = `handle_${handlerName}`;
      if (handlerModule && typeof handlerModule === 'object' && 
          handlerModule[handleName] && typeof handlerModule[handleName] === 'function') {
        functionName = handleName;
      } else {
        functionName = directName;
      }
    } else if (isMiddleware) {
      const snakeName = toSnakeCase(handlerName);
      functionName = `${snakeName}_middleware`;
    } else {
      const snakeName = toSnakeCase(handlerName);
      const handleSnake = `handle_${snakeName}`;
      const camelName = toCamelCase(handlerName);
      const handleCamel = `handle${camelName.charAt(0).toUpperCase() + camelName.slice(1)}`;
      
      if (handlerModule && typeof handlerModule === 'object') {
        if (handlerModule[handleSnake] && typeof handlerModule[handleSnake] === 'function') {
          functionName = handleSnake;
        } else if (handlerModule[handleCamel] && typeof handlerModule[handleCamel] === 'function') {
          functionName = handleCamel;
        } else {
          functionName = handleCamel; // Default
        }
      } else {
        functionName = handleCamel;
      }
    }
    
    if (typeof handlerModule === 'function') {
      handlerFn = handlerModule;
    } else if (handlerModule && typeof handlerModule === 'object') {
      if (handlerModule[functionName] && typeof handlerModule[functionName] === 'function') {
        handlerFn = handlerModule[functionName];
      } else {
        const camelCaseName = toCamelCase(handlerName);
        const handleNameUnderscore = `handle_${handlerName}`;
        const handleNameCamel = `handle${handlerName.charAt(0).toUpperCase() + handlerName.slice(1)}`;
        const handleNameCamelLower = `handle${camelCaseName.charAt(0).toUpperCase() + camelCaseName.slice(1)}`;
        
        if (handlerModule[handleNameUnderscore] && typeof handlerModule[handleNameUnderscore] === 'function') {
          handlerFn = handlerModule[handleNameUnderscore];
        } else if (handlerModule[handleNameCamel] && typeof handlerModule[handleNameCamel] === 'function') {
          handlerFn = handlerModule[handleNameCamel];
        } else if (handlerModule[handleNameCamelLower] && typeof handlerModule[handleNameCamelLower] === 'function') {
          handlerFn = handlerModule[handleNameCamelLower];
        } else if (handlerModule.handler && typeof handlerModule.handler === 'function') {
          handlerFn = handlerModule.handler;
        } else if (handlerModule.default && typeof handlerModule.default === 'function') {
          handlerFn = handlerModule.default;
        } else {
          const allKeys = [...new Set([...Object.keys(handlerModule), ...Object.getOwnPropertyNames(handlerModule)])];
          
          for (const key of allKeys) {
            if (key === '__esModule' || key.startsWith('Symbol(')) continue;
            
            if (key.startsWith('handle') && typeof handlerModule[key] === 'function') {
              handlerFn = handlerModule[key];
              break;
            }
          }
          if (!handlerFn) {
            for (const key of allKeys) {
              if (key === '__esModule' || key.startsWith('Symbol(')) continue;
              
              if (typeof handlerModule[key] === 'function') {
                handlerFn = handlerModule[key];
                break;
              }
            }
          }
        }
      }
    }
    
    if (!handlerFn) {
      const triedNames = [functionName, `handle_${handlerName}`, `handle${handlerName}`, 'handler', 'default'].join(', ');
      throw new Error(`Handler function not found in ${resolvedPath}. Tried: ${triedNames}, and all exports starting with 'handle'`);
    }
    
    const paramCount = handlerFn.length;
    
    const state = new State(handlerName, logFn);
    
    if (!state || !state.logger) {
      throw new Error(`State object not properly initialized. Has logger: ${!!state?.logger}`);
    }
    
    if (typeof state.logger.info !== 'function') {
      throw new Error(`Logger methods not available. Available methods: ${Object.getOwnPropertyNames(state.logger).join(', ')}`);
    }
    
    let result;
    
    if (isEventHandler) {
      const eventName = context.metadata?.event_name;
      if (!eventName) {
        throw new Error('Event name not found in context metadata');
      }
      
      const eventObj = await instantiateEventObject(eventName, context, projectRequire);
      
      if (paramCount >= 2) {
        result = await handlerFn(eventObj, state);
      } else {
        result = await handlerFn(eventObj);
      }
    } else if (isWebSocketHandler) {

      const wsResult = await callWebSocketHandler(
        handlerFn,
        context,
        paramCount,
        state,
        projectRequire
      );
      result = wsResult;
    } else if (isCronHandler) {
      if (paramCount === 0) {
        result = await handlerFn();
      } else if (paramCount === 1) {
        const minimalReq = context.payload || {};
        result = await handlerFn(minimalReq);
      } else if (paramCount >= 2) {
        const minimalReq = context.payload || {};
        result = await handlerFn(minimalReq, state);
      }
    } else if (paramCount >= 2) {
      const requestObj = await instantiateRequestObject(handlerName, context, projectRequire);
      result = await handlerFn(requestObj, state);
    } else if (paramCount >= 1) {
      const requestObj = await instantiateRequestObject(handlerName, context, projectRequire);
      result = await handlerFn(requestObj);
    } else {
      result = await handlerFn();
    }
    
    return {
      success: true,
      data: result,
      error: null,
      execution_time_ms: 0,
      _rohas_logs: logs,
      _rohas_triggers: state.getTriggers(),
      _rohas_auto_trigger_payloads: state.getAllAutoTriggerPayloads()
    };
    
  } catch (error) {
    return {
      success: false,
      data: null,
      error: error.message + '\n' + (error.stack || ''),
      execution_time_ms: 0,
      _rohas_logs: logs,
      _rohas_triggers: [],
      _rohas_auto_trigger_payloads: {}
    };
  }
}

async function handleRpcMessage(message) {
  try {
    const request = JSON.parse(message);
    
    if (request.method === 'execute') {
      const { handler_path, context } = request.params;
      const start = Date.now();
      
      const result = await executeHandler(handler_path, context);
      const execution_time_ms = Date.now() - start;
      
      return {
        jsonrpc: '2.0',
        id: request.id,
        result: {
          ...result,
          execution_time_ms
        }
      };
    } else if (request.method === 'ping') {
      return {
        jsonrpc: '2.0',
        id: request.id,
        result: { status: 'ok' }
      };
    } else if (request.method === 'shutdown') {
      process.exit(0);
    } else {
      return {
        jsonrpc: '2.0',
        id: request.id,
        error: {
          code: -32601,
          message: 'Method not found'
        }
      };
    }
  } catch (error) {
    return {
      jsonrpc: '2.0',
      id: null,
      error: {
        code: -32700,
        message: 'Parse error',
        data: error.message
      }
    };
  }
}

const originalLog = console.log;
const originalInfo = console.info;
console.log = (...args) => {
  process.stderr.write('[LOG] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n');
};
console.info = (...args) => {
  process.stderr.write('[INFO] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n');
};

async function main() {
  process.stdout.write(JSON.stringify({ type: 'ready' }) + '\n');
  
  rl.on('line', async (line) => {
    try {
      const response = await handleRpcMessage(line);
      process.stdout.write(JSON.stringify(response) + '\n');
    } catch (error) {
      const errorResponse = {
        jsonrpc: '2.0',
        id: null,
        error: {
          code: -32603,
          message: 'Internal error',
          data: error.message
        }
      };
      process.stdout.write(JSON.stringify(errorResponse) + '\n');
    }
  });
  
  process.on('SIGTERM', () => {
    process.exit(0);
  });
  
  process.on('SIGINT', () => {
    process.exit(0);
  });
}

main().catch((error) => {
  console.error('Worker startup error:', error);
  process.exit(1);
});

