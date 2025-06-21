# Summon DNAs

Central repository for all Holochain DNA source code for the Summon ecosystem.

## DNAs

### cart
Lightweight DNA containing carts, user profiles, addresses, and order fulfillment logic.

### products  
Large DNA containing the 30,000-item product catalog and user preferences.

## Architecture

These two DNAs are completely decoupled and do not communicate directly on the backend. All interaction is orchestrated by the frontend UI. This separation enables:

- Creation of lightweight shopper apps that only need cart_dna
- Independent versioning and deployment of backend components
- Clean separation of concerns between product catalog and order management

## Usage

This repository is used as a Git submodule in:
- `summon-customer-app` (Electron app)
- `summon-shopper-app` (Tauri mobile app)