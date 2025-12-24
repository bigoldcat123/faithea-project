import f from "./lib.js"
console.log("hello")
f()

// ,["abc"]
const ws = new WebSocket("/ws")
ws.onopen = () => {

}
ws.onerror = e => {
  console.log(e)
}
ws.onclose = e => {
  console.log("closed")
  console.log(e)
}

setInterval(() => {
  ws.send("hello")
},3000)
