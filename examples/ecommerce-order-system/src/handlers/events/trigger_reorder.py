from generated.state import State
from generated.events.inventory_low_stock import InventoryLowStock


async def handle_trigger_reorder(event: InventoryLowStock, state: State) -> None:
    """Trigger reorder process when inventory is low."""
    product_id = event.payload.get('productId')

    state.logger.info(f'Triggering reorder for product {product_id}')

    # In a real app, create purchase order or notify supplier
