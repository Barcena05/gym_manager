import { invoke } from '@tauri-apps/api/core';

export interface BarcodePdfItem {
	value: string;
	name?: string;
}

interface BarcodeImage {
	dataUrl: string;
	width: number;
	height: number;
}

async function renderBarcode(value: string): Promise<BarcodeImage> {
	const JsBarcode = (await import('jsbarcode')).default;
	const canvas = document.createElement('canvas');

	JsBarcode(canvas, value, {
		format: 'CODE128',
		width: 2,
		height: 70,
		displayValue: false,
		margin: 12,
		background: '#ffffff',
		lineColor: '#000000',
		valid: () => true
	});

	return {
		dataUrl: canvas.toDataURL('image/png'),
		width: canvas.width,
		height: canvas.height
	};
}

function fitWithin(width: number, height: number, maxWidth: number, maxHeight: number) {
	const scale = Math.min(maxWidth / width, maxHeight / height);
	return { width: width * scale, height: height * scale };
}

export async function buildBarcodePdfBase64(items: BarcodePdfItem[]): Promise<string> {
	const { jsPDF } = await import('jspdf');
	const doc = new jsPDF({ unit: 'mm', format: 'a4', orientation: 'portrait' });
	const pageWidth = 210;
	const pageHeight = 297;
	const margin = 10;

	if (items.length === 1) {
		const item = items[0];
		const barcode = await renderBarcode(item.value);
		const box = fitWithin(barcode.width, barcode.height, 160, 55);
		const centerX = pageWidth / 2;

		if (item.name) {
			doc.setFont('helvetica', 'bold');
			doc.setFontSize(13);
			const lines = doc.splitTextToSize(item.name, 170);
			doc.text(lines.slice(0, 2), centerX, 24, { align: 'center' });
		}

		doc.addImage(
			barcode.dataUrl,
			'PNG',
			centerX - box.width / 2,
			48,
			box.width,
			box.height
		);

		doc.setFont('helvetica', 'normal');
		doc.setFontSize(12);
		doc.text(item.value, centerX, 48 + box.height + 10, { align: 'center' });
	} else {
		const columns = 3;
		const labelWidth = (pageWidth - margin * 2) / columns;
		const labelHeight = 38;
		let x = margin;
		let y = margin;

		for (let i = 0; i < items.length; i += 1) {
			const item = items[i];
			const column = i % columns;

			if (i > 0 && column === 0) {
				x = margin;
				y += labelHeight;
			}

			if (y + labelHeight > pageHeight - margin) {
				doc.addPage();
				x = margin;
				y = margin;
			}

			const barcode = await renderBarcode(item.value);
			const box = fitWithin(barcode.width, barcode.height, labelWidth - 6, 20);
			const centerX = x + labelWidth / 2;

			if (item.name) {
				doc.setFont('helvetica', 'bold');
				doc.setFontSize(8);
				const nameLines = doc.splitTextToSize(item.name, labelWidth - 4);
				doc.text(nameLines[0] ?? '', centerX, y + 5, { align: 'center' });
			}

			doc.addImage(
				barcode.dataUrl,
				'PNG',
				centerX - box.width / 2,
				y + 8,
				box.width,
				box.height
			);

			doc.setFont('helvetica', 'normal');
			doc.setFontSize(9);
			doc.text(item.value, centerX, y + 33, { align: 'center' });

			x += labelWidth;
		}
	}

	const dataUri = doc.output('datauristring');
	return dataUri.substring(dataUri.indexOf(',') + 1);
}

export function isAndroid(): boolean {
	return typeof navigator !== 'undefined' && /Android/i.test(navigator.userAgent);
}

export async function savePdfAndOpen(base64: string): Promise<void> {
	if (isAndroid()) {
		// The Android plugin writes the PDF into the app cache itself and
		// opens it through FileProvider, avoiding path/root mismatches.
		await invoke('plugin:pdf|openPdf', { base64, path: '' });
		return;
	}

	const path = await invoke<string>('save_pdf', { base64 });
	await invoke('plugin:pdf|openPdf', { path });
}
