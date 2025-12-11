import asyncio
from generated.state import State


async def handle_sync_inventory(req, state: State) -> None:
    """
    Cron job to sync inventory and check for low stock.
    Runs every 6 hours.
    """
    state.logger.info('Running inventory sync cron job')

    # In a real app, check inventory levels from database
    await asyncio.sleep(0.3)  # 300ms - database query

    # Mock inventory check
    low_stock_products = [
        {'productId': 1, 'stock': 5, 'threshold': 10},
        {'productId': 2, 'stock': 2, 'threshold': 10},
    ]

    out_of_stock_products = [
        {'productId': 3, 'stock': 0},
    ]

    # Trigger low stock alerts
    for product in low_stock_products:
        state.logger.warning(f'Low stock alert: Product {product["productId"]} has {product["stock"]} units')
        state.trigger_event('InventoryLowStock', {
            'productId': product['productId'],
            'currentStock': product['stock'],
            'threshold': product['threshold']
        })

    # Trigger out of stock alerts
    for product in out_of_stock_products:
        state.logger.error(f'Out of stock: Product {product["productId"]}')
        state.trigger_event('InventoryOutOfStock', {
            'productId': product['productId']
        })

    state.logger.info('Inventory sync cron completed')
