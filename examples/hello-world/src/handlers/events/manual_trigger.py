import asyncio
from generated.state import State
from generated.events.manual_trigger import ManualTrigger

async def handle_manual_trigger(event: ManualTrigger, state: State) -> None:
    """Event handler for manual trigger events"""
    await asyncio.sleep(0.15)  # 150ms
    state.logger.info(f'Manual trigger processed: {event.payload}')
