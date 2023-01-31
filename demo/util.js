function log(text) {
  const output = document.getElementById('output');
  const p = document.createElement('p');
  p.innerText = text;
  output.appendChild(p);
}

function hex(array) {
  let hex = '';
  for (let byte of array) {
    hex += byte.toString(16);
  }
  return hex;
}
