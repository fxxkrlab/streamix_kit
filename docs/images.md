# Protected images

Image references are protected current-user resources. Send the original SXAPI Bearer in the Authorization header, including when using a player-specific image loader. A successful login, a copied image URL or a previous cache hit does not authorize a credential-free request.

Use image routes and bounds in the [client specification](../spec/README.md). Resolve root-relative references only on the trusted origin; never fetch provider URLs with user credentials. The SDK's `image` helper accepts media/library image routes and the exact person profile form `/sxapi/v1/persons/{uuid}/images/profile`, with a hard response cap, required current image capability, no redirects and checked content type. Person image availability depends on the matching RC.9 source and server acceptance; a contract path is not proof of a deployed photograph.

The SDK retains image-feature discovery from the login handshake instead of fetching capabilities again for every cover. That snapshot grants no access: each authenticated image request still checks live authorization on the server. A changed deployment may require refreshing the login/discovery; an HTTP denial is never bypassed through the snapshot.

A missing image can return 404. Show a placeholder; it does not mean that the media record or video file is missing. On temporary 429/503 apply a bounded, jittered retry policy in the UI and respect Retry-After when supplied. Do not create a retry storm, loosen authentication or cache one user's protected bytes for another user.

The application owns its authenticated image cache and invalidation when accounts change or access is revoked. No anonymous image compatibility path is added by StreamixKit.
