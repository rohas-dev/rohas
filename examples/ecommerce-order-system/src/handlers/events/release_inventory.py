import asyncio
from generated.state import State
from generated.events.order_cancelled import OrderCancelled


async def handle_release_inventory(event: OrderCancelled, state: State) -> None:
    """
    Release reserved inventory when order is cancelled.
    """
    order_id = event.payload.get('orderId')

    state.logger.info(f'Releasing inventory for cancelled order {order_id}')

    await asyncio.sleep(0.2)  # 200ms - database update

    # In a real app, release inventory in database
    state.trigger_event('InventoryReleased', {
        'orderId': order_id,
        'reason': 'order_cancelled'
    })

    state.logger.info(f'Inventory released for order {order_id}')
