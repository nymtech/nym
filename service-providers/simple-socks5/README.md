// connect to Nym local websocket

// Block and wait for incoming proxy requests. Each websocket message is a new request.

// When a request is detected, spawn a new task to deal with it

// Parse the address of the proxy request

// Parse the request_id of the proxy request

// Read the rest of the stream and use it as the body of the proxy request

// Connect to the remote machine

// Send the request body

// Listen for remote_response and save it when it comes back

// Temporary: set up a hardcoded  to_address. This will change once SURBs work.

// Concatenate the proxy_response as: to_address || request_id || remote_response

// Send the proxy_response back up the websocket