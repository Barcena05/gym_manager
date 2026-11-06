/**
 * Prints only the element with the given id.
 *
 * The element is cloned into a temporary `#print-root` appended to <body>
 * and the rest of the app is hidden with `body.printing`. This avoids
 * printing the application shell, browser headers/footers and the extra
 * blank pages caused by `visibility: hidden` on tall hidden content.
 */
export function printElementById(id: string): void {
	if (typeof document === 'undefined') {
		return;
	}

	const source = document.getElementById(id);
	if (!source) {
		window.print();
		return;
	}

	document.getElementById('print-root')?.remove();

	const root = document.createElement('div');
	root.id = 'print-root';
	root.appendChild(source.cloneNode(true));
	document.body.appendChild(root);
	document.body.classList.add('printing');

	let cleaned = false;
	const cleanup = () => {
		if (cleaned) return;
		cleaned = true;
		root.remove();
		document.body.classList.remove('printing');
		window.removeEventListener('afterprint', cleanup);
	};

	window.addEventListener('afterprint', cleanup);
	window.print();
}
