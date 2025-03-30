class CliCommandCard extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }

  connectedCallback() {
    this.render();
  }

  render() {
    const name = this.getAttribute('name') || '';
    const description = this.getAttribute('description') || '';
    const parent = this.getAttribute('parent') || '';
    const version = this.getAttribute('version') || '';
    
    const path = parent ? `${parent} ${name}` : name;

    this.shadowRoot.innerHTML = `
      <style>
        summary {
          font-weight: bold;
          cursor: pointer;
          padding: 0.5em;
          background-color: #333;
          color: #9eaaf6;
          box-shadow: 0 2px 2px rgba(0, 0, 0, 0.1);
          padding: 1rem 1.6rem;
        }
        summary::marker {
          color:rgb(199, 199, 199);
        }
        details {
          font-family: sans-serif;
          margin: 1em 0;
          border-radius: 6px;
          background-color: #363636; 
          color: #eee;
        }
        section {
          padding: 1.5em 2.2rem;
          display: flex;
          flex-direction: column;
          gap: 1em;
        }
        .label {
          font-weight: bold;
          margin-right: 0.5em;
        }
        .field-value {
          margin: 0 0.5em;
        }
        .slot-container {
          margin-top: 1em;
        }
      </style>
      <details open>
        <summary>${path}</summary>
        <section>
          <div><h4 class="label">Description:</h4><div class="field-value">${description}</div></div>
          ${!parent ? `<div><h4 class="label">Version:</h4><div class="field-value">${version}</div></div>` : ''}
          <div class="slot-container">
          <slot name="outputs"></slot>
          <slot name="children"></slot>
          <slot name="flags"></slot>
          <slot name="usage"></slot>
          <slot name="other"></slot>
          </div>
          ${parent.length ? `<div><h4 class="label">Parent:</h4><div class="field-value">${parent}</div></div>` : ''}
        </section>
      </details>
    `;
  }
}

customElements.define('cli-command-card', CliCommandCard);