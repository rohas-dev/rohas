import asyncio
from datetime import datetime
from generated.state import State


async def handle_expire_pending_orders(req, state: State) -> None:
    """
    Cron job to expire pending orders that haven't been paid.
    Runs every 5 minutes.
    """
    state.logger.info('Running order expiration cron job')

    # In a real app, query database for pending orders past expiration
    # For demo, we'll simulate finding expired orders
    await asyncio.sleep(0.2)  # 200ms - database query

    # Mock expired orders
    expired_orders = [
        {'orderId': 1, 'expiresAt': '2024-01-01T00:00:00'},
        {'orderId': 2, 'expiresAt': '2024-01-01T00:05:00'},
    ]

    for order in expired_orders:
        order_id = order['orderId']
        state.logger.info(f'Expiring order {order_id}')

        # Trigger OrderExpired event
        state.trigger_event('OrderExpired', {
            'orderId': order_id,
            'expiredAt': datetime.now().isoformat()
        })

    state.logger.info(f'Order expiration cron completed, processed {len(expired_orders)} orders')
