from generated.websockets.hello_world import HelloWorldMessage, HelloWorldConnection
from generated.state import State
from typing import Optional


async def on_message_handler(message: HelloWorldMessage, connection: HelloWorldConnection, state: State) -> Optional[dict]:
    print(f'Received message: {message.data.payload}')

    from datetime import datetime
    from generated.dto.web_socket_message import WebSocketMessage

    response_data = WebSocketMessage(
        type="echo",
        payload={
            "original": message.data.dict() if hasattr(message.data, "dict") else str(message.data),
            "response": "Message received successfully!"
        },
    )

    response = HelloWorldMessage(
        data=response_data,
        timestamp=datetime.now(),
    )

    return {
        "data": response.data.model_dump(),
        "timestamp": response.timestamp.isoformat(),
    }
