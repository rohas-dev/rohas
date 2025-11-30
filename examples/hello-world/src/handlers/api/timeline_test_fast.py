import asyncio
from generated.state import State
from generated.api.timeline_test_fast import TimelineTestFastRequest, TimelineTestFastResponse

async def handle_timeline_test_fast(req: TimelineTestFastRequest, state: State) -> TimelineTestFastResponse:
    """Fast API handler - completes in ~50ms"""
    await asyncio.sleep(0.05)  # 50ms
    
    state.logger.info('Fast handler completed')
    state.trigger_event('FastCompleted', {'duration': 50})
    
    return TimelineTestFastResponse(data="Fast operation completed in ~50ms")

