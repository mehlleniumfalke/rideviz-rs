import { fireEvent, render, screen } from '@testing-library/react';
import ColorByPicker from './ColorByPicker';

describe('ColorByPicker', () => {
  it('disables unavailable options and emits available selection', () => {
    const onChange = vi.fn();
    render(
      <ColorByPicker
        value={null}
        gradient="rideviz"
        availableData={{ has_elevation: true, has_heart_rate: false, has_power: false }}
        onChange={onChange}
      />,
    );

    const heartRateButton = screen.getByRole('button', { name: 'Heart Rate' });
    expect(heartRateButton).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: 'Elevation' }));
    expect(onChange).toHaveBeenCalledWith('elevation');
  });
});
