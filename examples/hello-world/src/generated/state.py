from typing import Any, Dict, List, Optional
from pydantic import BaseModel


class TriggeredEvent(BaseModel):
    event_name: str
    payload: Dict[str, Any]


class State:
    """Context object for handlers to trigger events and access runtime state."""
    
    def __init__(self):
        self._triggers: List[TriggeredEvent] = []
        self._auto_trigger_payloads: Dict[str, Dict[str, Any]] = {}
    
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
