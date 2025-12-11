from generated.state import State
from generated.events.inventory_released import InventoryReleased


async def handle_log_inventory_release(event: InventoryReleased, state: State) -> None:
    """Log inventory release for audit purposes."""
    order_id = event.payload.get('orderId')
    reason = event.payload.get('reason')

    state.logger.info(f'Inventory release logged for order {order_id}, reason: {reason}')
