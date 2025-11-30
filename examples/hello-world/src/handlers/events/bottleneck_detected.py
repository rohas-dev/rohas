import asyncio
from generated.state import State
from generated.events.bottleneck_detected import BottleneckDetected

async def handle_bottleneck_detected(event: BottleneckDetected, state: State) -> None:
    """Event handler for bottleneck detection - logs the bottleneck"""
    await asyncio.sleep(0.1)  # 100ms
    state.logger.warning(f'Bottleneck detected: {event.payload}')
    state.trigger_event('BottleneckLogged', {'operation': event.payload.get('operation', 'unknown')})

