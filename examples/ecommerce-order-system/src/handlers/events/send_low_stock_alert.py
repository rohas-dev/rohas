import asyncio
from generated.state import State
from generated.events.inventory_low_stock import InventoryLowStock


async def handle_send_low_stock_alert(event: InventoryLowStock, state: State) -> None:
    """Send alert when inventory is low."""
    product_id = event.payload.get('productId')
    current_stock = event.payload.get('currentStock')
    threshold = event.payload.get('threshold')

    state.logger.warning(f'Low stock alert: Product {product_id} has {current_stock} units (threshold: {threshold})')

    await asyncio.sleep(0.1)
    # In a real app, send email/notification to warehouse manager
