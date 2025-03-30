document.addEventListener('DOMContentLoaded', async () => {
  const response = await fetch('./cli-structure.json');
  if (!await response.ok) {
    console.error('Failed to fetch CLI data:', response.statusText);
    return;
  }
  const cliData = await response.json();

  const container = document.querySelector('#app-container');
  if (!container) {
    console.error('No app-container found in the document.');
    return;
  }

  renderCommands(container, cliData);
})

function createCard(command, parent = '') {
  const el = document.createElement('cli-command-card');
  el.setAttribute('name', command.name || '');
  el.setAttribute('description', command.description || '');
  el.setAttribute('parent', parent);
  el.setAttribute('version', command.version || '');

  // Outputs slot
  if (command.outputs) {
    const outputs = document.createElement('div');
    outputs.setAttribute('slot', 'outputs');
    const rawText = command.outputs.help_page?.stdout || '';
    outputs.innerHTML = `
      <details open="open">
        <summary style="margin-bottom: .7rem; cursor: pointer;">Help Page</summary>
        <pre style="margin: 0; padding: 0 1rem; background-color: #333; color: #f7f7f7; line-height: 1.5">
          <code style="white-space: pre-wrap;">
${rawText}
          </code>
        </pre>
      </details>
    `;
    el.appendChild(outputs);
  }

  // Flags slot
  if (command.children?.FLAG?.length) {
    const flags = document.createElement('div');
    console.log(command.children.FLAG.map(f => {
      console.log(f);
    }));
    flags.setAttribute('slot', 'flags');
    flags.innerHTML = `
    <h4>Flags</h4>
      <table>
        <thead style="background-color: #333; height: 2.5rem;">
          <tr style="color: #f7f7f7;" >
            <th scope="col" style="min-width: max-content; text-align: start; padding: 0 1rem;">Short</th>
            <th scope="col" style="min-width: max-content; text-align: start; padding: 0 1rem;">Long</th>
            <th scope="col" style="min-width: max-content; text-align: start; padding: 0 1rem;">Data Type</th>
            <th scope="col" style="min-width: max-content; text-align: start; padding: 0 1rem;">Description</th>
        </thead>
        <tbody>` +
      command.children.FLAG.map(f => `<tr style="height: 1.5rem;"><td style="text-align: start; padding: 0 1rem">${f.short || ''}</td><td style="text-align: start; padding: 0 1rem">${f.long || ''}</td><td style="text-align: start; padding: 0 1rem">${f.data_type || 'boolean'}</td><td style="text-align: start; padding: 0 1rem">${f.description || ''}</td></tr>`).join('') +
      `</tbody></table>`;
    el.appendChild(flags);
  }

  // Other slot
  // console.log(command.children?.OTHER);
  // if (command.children?.OTHER?.length) {
  //   const other = document.createElement('div');
  //   other.setAttribute('slot', 'other');
  //   other.innerHTML = `<h4>Other</h4><ul>` +
  //     command.children.OTHER.map(o => `<li>${o.contents}</li>`).join('') +
  //     `</ul>`;
  //   el.appendChild(other);
  // }

  return el;
}

function flattenCommands(node, parent = '', depth = 0, acc = [], path = '') {
  const fullPath = path ? `${path} ${node.name}`.trim() : node.name;
  acc.push({ node, parent: path, depth, path: fullPath });
  const children = node.children?.COMMAND || {};
  for (const key in children) {
    flattenCommands(children[key], node.name, depth + 1, acc, fullPath);
  }
  return acc;
}

function groupByFullParent(flatList) {
  const groups = {};
  for (const item of flatList) {
    const groupKey = item.path.split(' ').slice(0, -1).join(' ') || 'root';
    if (!groups[groupKey]) groups[groupKey] = [];
    groups[groupKey].push(item);
  }
  return groups;
}

function renderCommands(container, cliData) {
  const flat = flattenCommands(cliData);
  flat.sort((a, b) => a.depth - b.depth);

  const grouped = {};

  for (const entry of flat) {
    if (entry.depth === 1) {
      // Direct child of root: this becomes its own group header
      const groupKey = entry.path;
      grouped[groupKey] = [entry];
    } else if (entry.depth > 1) {
      const parentPath = entry.path.split(' ').slice(0, -1).join(' ');
      if (!grouped[parentPath]) grouped[parentPath] = [];
      grouped[parentPath].push(entry);
    }
  }

  for (const groupPath in grouped) {
    const section = document.createElement('section');
    const header = document.createElement('h2');
    header.textContent = groupPath;
    section.appendChild(header);

    const sortedGroup = grouped[groupPath].sort((a, b) => {
      if (a.path === groupPath) return -1;
      if (b.path === groupPath) return 1;
      return a.depth - b.depth;
    });

    sortedGroup.forEach(({ node, parent }) => {
      const card = createCard(node, parent);
      section.appendChild(card);
    });

    container.appendChild(section);
  }
}