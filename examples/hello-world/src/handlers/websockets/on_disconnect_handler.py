from generated.websockets.hello_world import HelloWorldMessage, HelloWorldConnection
from generated.state import State
from typing import Optional

async def on_disconnect_handler(connection: HelloWorldConnection, state: State) -> None:
    # TODO: Implement onDisconnect handler
    print(f'Client disconnected: {connection.connection_id}')
