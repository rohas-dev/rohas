from generated.websockets.hello_world import HelloWorldMessage, HelloWorldConnection
from generated.state import State
from typing import Optional

async def on_connect_handler(connection: HelloWorldConnection, state: State) -> Optional[HelloWorldMessage]:
    print(f'Client connected: {connection.connection_id}')
    # Send a welcome message
    from datetime import datetime
    from generated.dto.web_socket_message import WebSocketMessage
    return HelloWorldMessage(
        data=WebSocketMessage(
            type="welcome",
            payload={"message": f"Hello! You are connected as {connection.connection_id}"}
        ),
        timestamp=datetime.now()
    )
