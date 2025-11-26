from dataclasses import dataclass
from typing import Optional
from datetime import datetime

@dataclass
class User:
    id: int
    name: str
    email: str
    createdAt: datetime
