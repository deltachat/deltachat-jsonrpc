// minimal h
export function h(type, content, css_class) {
  const node = document.createElement(type);
  let hcontent = (c) => {
    if (c instanceof Node) {
      node.appendChild(c);
    } else if (typeof c == "string") {
      node.append(document.createTextNode(c));
    }
  };
  if (Array.isArray(content)) {
    content.map(hcontent);
  } else {
    hcontent(content);
  }

  if (css_class) node.classList.add(...css_class.split(" "));
  return node;
}
