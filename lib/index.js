const rustBridge = require('../native');

const handle = rustBridge.fxaNew();
const url = rustBridge.fxaBeginOAuthFlow(handle);
console.log("GO TO:")
console.log(url);
let success = rustBridge.fxaCompleteOAuthFlow(handle, "bobo", "bibi");
if (success) {
    console.log("YAY!");
}
