import asyncio
from generated.state import State
from generated.api.timeline_test_very_slow import TimelineTestVerySlowRequest, TimelineTestVerySlowResponse

async def handle_timeline_test_very_slow(req: TimelineTestVerySlowRequest, state: State) -> TimelineTestVerySlowResponse:
    """Very slow API handler - completes in ~6s (major bottleneck)"""
    await asyncio.sleep(2.0)  # 2s - heavy computation
    state.logger.info('Heavy computation completed')

    await asyncio.sleep(2.5)  # 2.5s - file processing
    state.logger.info('File processing completed')

    await asyncio.sleep(1.5)  # 1.5s - final processing
    state.logger.info('Final processing completed')

    state.trigger_event('VerySlowCompleted', {'duration': 6000})
    state.trigger_event('MajorBottleneckDetected', {'operation': 'timeline_test_very_slow'})

    return TimelineTestVerySlowResponse(data="Very slow operation completed in ~6s")

