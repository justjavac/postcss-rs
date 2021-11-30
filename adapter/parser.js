const fs = require('fs');
const parse = require("postcss/lib/parse");

const file_list = [
  ["tailwind-components.css", "2.8K"],
  ["bootstrap-reboot.css", "7.4K"],
  ["bootstrap-grid.css", "71K"],
  ["bootstrap.css", "201K"],
  ["tailwind.css", "3.5M"],
  ["tailwind-dark.css", "5.8M"],
];

for ([file, size] of file_list) {
  const css = fs.readFileSync(`../assets/${file}`).toString();
  const tag = `js: parser/${file}(${size})`;
  console.time(tag);
  parse(css, { map: false });
  console.timeEnd(tag);
}
