import asyncio
from generated.state import State
from generated.api.timeline_test_slow import TimelineTestSlowRequest, TimelineTestSlowResponse

async def handle_timeline_test_slow(req: TimelineTestSlowRequest, state: State) -> TimelineTestSlowResponse:
    """Slow API handler - completes in ~2s (bottleneck)"""
    await asyncio.sleep(0.5)  # 500ms - database query simulation
    state.logger.info('Database query completed')

    await asyncio.sleep(0.8)  # 800ms - external API call
    state.logger.info('External API call completed')

    await asyncio.sleep(0.7)  # 700ms - processing
    state.logger.info('Processing completed')

    state.trigger_event('SlowCompleted', {'duration': 2000})
    state.trigger_event('BottleneckDetected', {'operation': 'timeline_test_slow'})

    return TimelineTestSlowResponse(data="Slow operation completed in ~2s")

