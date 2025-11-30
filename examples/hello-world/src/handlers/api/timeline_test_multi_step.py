import asyncio
from generated.state import State
from generated.api.timeline_test_multi_step import TimelineTestMultiStepRequest, TimelineTestMultiStepResponse

async def handle_timeline_test_multi_step(req: TimelineTestMultiStepRequest, state: State) -> TimelineTestMultiStepResponse:
    """Multi-step handler with multiple events and varying durations"""
    # Step 1: Fast validation
    await asyncio.sleep(0.1)  # 100ms
    state.logger.info('Validation completed')
    state.trigger_event('ValidationComplete', {'step': 1})

    # Step 2: Medium processing
    await asyncio.sleep(0.5)  # 500ms
    state.logger.info('Processing completed')
    state.trigger_event('ProcessingComplete', {'step': 2})

    # Step 3: Slow external call
    await asyncio.sleep(1.2)  # 1200ms - bottleneck
    state.logger.info('External call completed')
    state.trigger_event('ExternalCallComplete', {'step': 3})

    # Step 4: Fast finalization
    await asyncio.sleep(0.2)  # 200ms
    state.logger.info('Finalization completed')
    state.trigger_event('FinalizationComplete', {'step': 4})

    return TimelineTestMultiStepResponse(data="Multi-step operation completed")

