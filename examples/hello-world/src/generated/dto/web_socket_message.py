from pydantic import BaseModel
from typing import Optional
from datetime import datetime

class WebSocketMessage(BaseModel):
    type: str
    payload: dict

    class Config:
        from_attributes = True
