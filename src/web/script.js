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
      <details>
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

  // Children slot - all children become nested cli-command-card elements
  if (command.children?.COMMAND && Object.keys(command.children.COMMAND).length > 0) {
    const childrenContainer = document.createElement('div');
    childrenContainer.setAttribute('slot', 'children');
    childrenContainer.innerHTML = '<h4>Subcommands</h4>';
    
    const fullPath = parent ? `${parent} ${command.name}` : command.name;
    
    for (const [childName, childCommand] of Object.entries(command.children.COMMAND)) {
      // Create a nested card for all child commands
      const nestedWrapper = document.createElement('div');
      nestedWrapper.style.marginLeft = '1rem';
      nestedWrapper.style.borderLeft = '2px solid #555';
      nestedWrapper.style.paddingLeft = '1rem';
      nestedWrapper.style.marginTop = '1rem';
      
      const childCard = createCard(childCommand, fullPath);
      nestedWrapper.appendChild(childCard);
      childrenContainer.appendChild(nestedWrapper);
    }
    
    el.appendChild(childrenContainer);
  }

  return el;
}


function renderCommands(container, cliData) {
  // Render top-level commands as section headers with their children as cards
  const topLevelCommands = cliData.children?.COMMAND || {};
  
  for (const [commandName, commandData] of Object.entries(topLevelCommands)) {
    const section = document.createElement('section');
    const header = document.createElement('h2');
    header.textContent = `${cliData.name} ${commandName}`;
    section.appendChild(header);
    
    // Add description of the top-level command
    if (commandData.description) {
      const description = document.createElement('p');
      description.textContent = commandData.description;
      description.style.color = '#ccc';
      description.style.fontStyle = 'italic';
      description.style.marginBottom = '1rem';
      section.appendChild(description);
    }
    
    // Render children of top-level command as cards
    const children = commandData.children?.COMMAND || {};
    const fullPath = `${cliData.name} ${commandName}`;
    
    for (const [childName, childCommand] of Object.entries(children)) {
      const card = createCard(childCommand, fullPath);
      section.appendChild(card);
    }
    
    container.appendChild(section);
  }
}