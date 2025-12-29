import f from "./lib.js"
console.log("hello")
f()

// ,["abc"]
const ws = new WebSocket("/ws/dadigua")
ws.onopen = () => {

}
ws.onmessage = e => {
  console.log(e.data)
}
ws.onerror = e => {
  console.log(e)
}
ws.onclose = e => {
  console.log("closed")
  console.log(e)
}
let a = 0;
setInterval(() => {
  // let a = "";
  // for (let i = 0; i < 126; i++) {
  //   a += "a"
  // }
  // console.log(a)
  a += 1;
  ws.send("hello world" + a)
},1000)
