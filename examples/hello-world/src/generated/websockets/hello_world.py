from pydantic import BaseModel
from typing import Dict, Any, Optional
from datetime import datetime

from ..dto.web_socket_message import WebSocketMessage
class HelloWorldMessage(BaseModel):
    data: WebSocketMessage
    timestamp: datetime

    class Config:
        from_attributes = True

class HelloWorldConnection(BaseModel):
    connection_id: str
    path: str
    connected_at: datetime

    class Config:
        from_attributes = True
