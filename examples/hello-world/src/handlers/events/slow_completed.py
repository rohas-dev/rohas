import asyncio
from generated.state import State
from generated.events.slow_completed import SlowCompleted

async def handle_slow_completed(event: SlowCompleted, state: State) -> None:
    """Event handler for slow completed events - takes ~800ms"""
    await asyncio.sleep(0.3)  # 300ms
    state.logger.info('Processing slow event...')

    await asyncio.sleep(0.5)  # 500ms - additional processing
    state.logger.info(f'Slow event processed: {event.payload}')

