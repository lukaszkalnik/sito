import { LicensesDialog } from './features/about/LicensesDialog';
import { useLicensesDialog } from './features/about/useLicenses';
import { ScanScreen } from './features/scan/ScanScreen';
import { UpdateNotice } from './features/update/UpdateNotice';

function App() {
  const licenses = useLicensesDialog();
  return (
    <>
      <ScanScreen />
      <LicensesDialog open={licenses.open} state={licenses.state} onClose={licenses.close} />
      <UpdateNotice />
    </>
  );
}

export default App;
