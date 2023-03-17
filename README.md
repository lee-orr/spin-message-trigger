# Spin Message Trigger

A generiuc messaging trigger for (Fermyon Spin)[https://github.com/fermyon/spin]. Note this is an experimental/work-in-progress repo, and is not production ready at this point. Also - if you have suggestions for improvements, feel free to make them in the Issues tab.

## Features

- Use multipe message brokers, including (currently):
    - a simple in memory broker (with * based wildcard support)
    - redis pubsub
    - NATS
- Named brokers - allowing multiple brokers of the same or different types in a single application
    - this is designed to support usecases such as separating the publishing of internal domain events & public events meant for others to consume
- an HTTP gateway server for publishing messages to the broker
- a WebSocket server allowing subscribing to single subjects (with wildcard support, as available for the broker)
- Trigger a Spin component from a message on a subscribed channel
- Publish messages from the Spin component - defaulting to the same broker & subject, but optionally setting other defaults or manually setting the broker & subject for each message

### Desired Features
- Support for some request-response paradigms
- Additional broker support (possibly utilizing TriggerMesh)
- Support for basic event-sourcing structures

## Installation
This is a WIP

## Usage
This is a WIP

## Development
This repository is set up to function as either a Dev Container (using VsCode) or a Docker Dev Environment. This means you can use Github workspaces to get it set up automatically, or use VSCodes "Clone Repository into Volume" option to clone the repo & build the dev environment for you.

Once you are running, you can use the "update-plugin.sh" script to build the plugin and install it on spin.
Then - go to the example app folder and run "spin up" to launch it.

If you are using VS Code, you'll notice there are two extensions installed:
- Thunder client, which lets you make HTTP requests.
    - you should see, under "Collections" an example app collection with an example request
- WebSocket, which lets you connect to websockets.
    - you should be able to connect to "ws://localhost:3006/subscribe/good.*"
    - When you send the example request in thunder client, you should see a "Goodbye" message arrive in the websocket client

### Repo Structure
- The `trigger-message` crate contains the spin plugin, including the message broker interfaces
- The `spin-message-types` crate contains the `.wit` definition, some shared file types to allow for easy creation of messages, and a `#[message_component]` macro for setting up the trigger in your app
- The example app contains a single wasm file, demonstrating simple use, and showcasing the use of multiple brokers within a single file