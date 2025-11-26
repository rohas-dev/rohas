from dataclasses import dataclass
from typing import Optional
from datetime import datetime

@dataclass
class CreateUserInput:
    name: str
    email: str
