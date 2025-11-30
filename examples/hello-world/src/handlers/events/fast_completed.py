import asyncio
from generated.state import State
from generated.events.fast_completed import FastCompleted

async def handle_fast_completed(event: FastCompleted, state: State) -> None:
    """Event handler for fast completed events"""
    await asyncio.sleep(0.03)  # 30ms
    state.logger.info(f'Fast event processed: {event.payload}')

