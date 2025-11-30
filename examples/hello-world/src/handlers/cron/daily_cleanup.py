import asyncio
from generated.state import State

async def handle_daily_cleanup(req, state: State) -> None:
    """Cron handler with multiple steps for timeline testing"""
    state.logger.info('Starting daily cleanup')

    # Step 1: Clean old records
    await asyncio.sleep(0.3)  # 300ms
    state.logger.info('Cleaned old records')
    state.trigger_event('CleanupStep1', {'step': 'old_records'})

    # Step 2: Archive data
    await asyncio.sleep(0.8)  # 800ms
    state.logger.info('Archived data')
    state.trigger_event('CleanupStep2', {'step': 'archive'})

    # Step 3: Update statistics
    await asyncio.sleep(0.2)  # 200ms
    state.logger.info('Updated statistics')

    state.logger.info('Daily cleanup completed')
