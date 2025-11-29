from typing import Any, Dict, List, Optional
from pydantic import BaseModel


class TriggeredEvent(BaseModel):
    event_name: str
    payload: Dict[str, Any]


class Logger:
    """Logger for handlers to emit structured logs."""
    
    def __init__(self, handler_name: str, log_fn: Any):
        self._handler_name = handler_name
        self._log_fn = log_fn
    
    def info(self, message: str, **kwargs: Any) -> None:
        """Log an info message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("info", self._handler_name, message, kwargs)
    
    def error(self, message: str, **kwargs: Any) -> None:
        """Log an error message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("error", self._handler_name, message, kwargs)
    
    def warning(self, message: str, **kwargs: Any) -> None:
        """Log a warning message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("warn", self._handler_name, message, kwargs)
    
    def warn(self, message: str, **kwargs: Any) -> None:
        """Log a warning message (alias for warning).
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        self.warning(message, **kwargs)
    
    def debug(self, message: str, **kwargs: Any) -> None:
        """Log a debug message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("debug", self._handler_name, message, kwargs)
    
    def trace(self, message: str, **kwargs: Any) -> None:
        """Log a trace message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("trace", self._handler_name, message, kwargs)


class State:
    """Context object for handlers to trigger events and access runtime state."""
    
    def __init__(self, handler_name: Optional[str] = None, log_fn: Optional[Any] = None):
        self._triggers: List[TriggeredEvent] = []
        self._auto_trigger_payloads: Dict[str, Dict[str, Any]] = {}
        self.logger = Logger(handler_name or "unknown", log_fn)
    
    def trigger_event(self, event_name: str, payload: Dict[str, Any]) -> None:
        """Manually trigger an event with the given payload.
        
        Use this for events that are NOT defined in the schema's triggers list.
        
        Args:
            event_name: Name of the event to trigger
            payload: Event payload data (will be serialized to JSON)
        """
        self._triggers.append(TriggeredEvent(
            event_name=event_name,
            payload=payload
        ))
    
    def set_payload(self, event_name: str, payload: Dict[str, Any]) -> None:
        """Set the payload for an auto-triggered event.
        
        Use this for events that ARE defined in the schema's triggers list.
        The event will be automatically triggered after the handler completes,
        using the payload you set here.
        
        Args:
            event_name: Name of the event (must match a trigger in schema)
            payload: Event payload data (will be serialized to JSON)
        """
        self._auto_trigger_payloads[event_name] = payload
    
    def get_triggers(self) -> List[TriggeredEvent]:
        """Get all manually triggered events. Used internally by the runtime."""
        return self._triggers.copy()
    
    def get_auto_trigger_payload(self, event_name: str) -> Optional[Dict[str, Any]]:
        """Get payload for an auto-triggered event. Used internally by the runtime."""
        return self._auto_trigger_payloads.get(event_name)
    
    def get_all_auto_trigger_payloads(self) -> Dict[str, Dict[str, Any]]:
        """Get all auto-trigger payloads. Used internally by the runtime."""
        return self._auto_trigger_payloads.copy()
