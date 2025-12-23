import f from "./lib.js"
console.log("hello")
f()


const ws = new WebSocket("/ws")
ws.onopen = () => {

}
ws.onerror = e => {
  console.log(e)
}
