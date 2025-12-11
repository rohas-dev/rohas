from generated.state import State
from generated.events.inventory_reserved import InventoryReserved


async def handle_log_inventory_reservation(event: InventoryReserved, state: State) -> None:
    """Log inventory reservation for audit purposes."""
    order_id = event.payload.get('orderId')
    items = event.payload.get('items', [])

    state.logger.info(f'Inventory reservation logged for order {order_id}: {len(items)} items')
