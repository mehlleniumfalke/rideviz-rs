import { fireEvent, render, screen } from '@testing-library/react';
import GradientPicker from './GradientPicker';

describe('GradientPicker', () => {
  it('emits selected gradient', () => {
    const onChange = vi.fn();
    render(<GradientPicker selectedGradient="fire" onChange={onChange} />);

    fireEvent.click(screen.getByLabelText('Select ocean gradient'));
    expect(onChange).toHaveBeenCalledWith('ocean');
  });
});
